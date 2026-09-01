use crate::db::{accounts_all, categories_all, pockets_all, transactions_all, AppState};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub success: bool,
    pub message: String,
    pub pushed: u32,
}

pub async fn push_to_sheet(
    sheet_url: &str,
    state: &tauri::State<'_, AppState>,
) -> Result<SyncResult, String> {
    if sheet_url.is_empty() {
        return Err("URL Google Spreadsheet belum diatur".into());
    }
    // Collect pending transactions + full domain data (accounts/savings/categories)
    // while holding the lock, then drop the lock before any network await.
    let (payload, accounts_payload, savings_payload, categories_payload) =
        {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let txs = transactions_all(&db, None, None, None, None, None, None, None, None)
                .unwrap_or_default();
            let tx_payload: Vec<serde_json::Value> = txs
                .iter()
                .filter(|t| t.sync_status != crate::models::SyncStatus::Synced)
                .map(|t| serde_json::json!({
                    "id": t.id, "date": t.date, "type": t.transaction_type.as_str(),
                    "account_id": t.account_id, "destination_account_id": t.destination_account_id,
                    "category_id": t.category_id, "amount": t.amount, "note": t.note,
                    "sync_status": t.sync_status.as_str()
                }))
                .collect();

            // Map Accounts to the sheet schema: current_balance -> balance, is_active bool -> 0/1.
            let accs = accounts_all(&db).unwrap_or_default();
            let acc_payload: Vec<serde_json::Value> = accs
                .iter()
                .map(|a| serde_json::json!({
                    "id": a.id,
                    "name": a.name,
                    "account_type": a.account_type.as_str(),
                    "balance": a.current_balance,
                    "is_active": if a.is_active { 1 } else { 0 },
                }))
                .collect();

            // Map SavingsPocket to sheet schema: { id, name, target_amount, current_amount, linked_account_id }.
            let pockets = pockets_all(&db).unwrap_or_default();
            let sav_payload: Vec<serde_json::Value> = pockets
                .iter()
                .map(|p| serde_json::json!({
                    "id": p.id,
                    "name": p.name,
                    "target_amount": p.target_amount,
                    "current_amount": p.current_amount,
                    "linked_account_id": p.linked_account_id,
                }))
                .collect();

            // Map Category to sheet schema: color_hex -> color.
            let cats = categories_all(&db).unwrap_or_default();
            let cat_payload: Vec<serde_json::Value> = cats
                .iter()
                .map(|c| serde_json::json!({
                    "id": c.id,
                    "name": c.name,
                    "type": c.category_type.as_str(),
                    "icon": c.icon,
                    "color": c.color_hex,
                }))
                .collect();

            (tx_payload, acc_payload, sav_payload, cat_payload)
        };

    if payload.is_empty() && accounts_payload.is_empty() && savings_payload.is_empty() && categories_payload.is_empty() {
        return Ok(SyncResult {
            success: true,
            message: "Tidak ada data yang perlu disinkronisasi".into(),
            pushed: 0,
        });
    }

    let count = payload.len() as u32;
    let body = serde_json::json!({ "action": "push", "transactions": payload });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(sheet_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Koneksi ke Google Sheet gagal: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Google Sheet merespons {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        ));
    }

    // Sync reference sheets separately so legacy transaction push stays compatible;
    // failures here remain non-fatal because transaction push already succeeded.
    let sync_all_body = serde_json::json!({
        "action": "syncAll",
        "data": {
            // Null prevents Code.gs `if (data.transactions)` from clearing Transactions.
            "transactions": serde_json::Value::Null,
            "accounts": accounts_payload,
            "savings": savings_payload,
            "categories": categories_payload,
        }
    });
    let sync_all_resp = client.post(sheet_url).json(&sync_all_body).send().await;
    if let Ok(response) = sync_all_resp {
        if !response.status().is_success() {
            eprintln!("Spreadsheet reference-data sync failed: {}", response.status());
        }
    } else {
        eprintln!("Spreadsheet reference-data sync request failed");
    }

    Ok(SyncResult {
        success: true,
        message: format!("{count} transaksi berhasil dikirim"),
        pushed: count,
    })
}

/// Parse a sheet cell as integer, tolerating numeric strings from text-formatted cells.
fn sheet_int(v: &serde_json::Value) -> i64 {
    match v {
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(0),
        serde_json::Value::String(s) => s.replace(',', "").parse().unwrap_or(0),
        _ => 0,
    }
}

pub async fn fetch_from_sheet(
    sheet_url: &str,
    state: &tauri::State<'_, crate::db::AppState>,
) -> Result<serde_json::Value, String> {
    if sheet_url.is_empty() {
        return Err("URL Google Spreadsheet belum diatur".into());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;

    let url = if sheet_url.contains('?') {
        format!("{sheet_url}&action=fetch")
    } else {
        format!("{sheet_url}?action=fetch")
    };

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Fetch gagal: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "Sheet merespons {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        ));
    }
    let data = resp
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?;
    let payload = data.get("data").unwrap_or(&data);

    // Merge reference data first so transaction foreign keys remain valid locally.
    {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        let now = chrono::Utc::now().timestamp();

        // Accounts use sheet balance/is_active names and upsert all mutable fields.
        if let Some(arr) = payload.get("accounts").and_then(|v| v.as_array()) {
            for item in arr {
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                if id.is_empty() { continue; }
                let _ = db.execute(
                    "INSERT INTO accounts(id,name,account_type,account_number,current_balance,is_active,created_at,updated_at) VALUES(?1,?2,?3,NULL,?4,?5,?6,?6) ON CONFLICT(id) DO UPDATE SET name=excluded.name,account_type=excluded.account_type,current_balance=excluded.current_balance,is_active=excluded.is_active,updated_at=excluded.updated_at",
                    rusqlite::params![id, item.get("name").and_then(|v| v.as_str()).unwrap_or(""), item.get("account_type").and_then(|v| v.as_str()).unwrap_or("bank"), sheet_int(item.get("balance").unwrap_or(&serde_json::Value::Null)), item.get("is_active").map(|v| v.as_bool().unwrap_or(v.as_i64().unwrap_or(0) != 0)).unwrap_or(true), now],
                );
            }
        }

        // Categories use sheet type/color names and upsert all mutable fields.
        if let Some(arr) = payload.get("categories").and_then(|v| v.as_array()) {
            for item in arr {
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                if id.is_empty() { continue; }
                let _ = db.execute(
                    "INSERT INTO categories(id,name,category_type,icon,color_hex) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET name=excluded.name,category_type=excluded.category_type,icon=excluded.icon,color_hex=excluded.color_hex",
                    rusqlite::params![id, item.get("name").and_then(|v| v.as_str()).unwrap_or(""), item.get("type").and_then(|v| v.as_str()).unwrap_or("expense"), item.get("icon").and_then(|v| v.as_str()).unwrap_or("wallet"), item.get("color").and_then(|v| v.as_str()).unwrap_or("#6366f1")],
                );
            }
        }

        // Savings must follow accounts because linked_account_id is a foreign key.
        if let Some(arr) = payload.get("savings").and_then(|v| v.as_array()) {
            for item in arr {
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let linked = item.get("linked_account_id").and_then(|v| v.as_str()).unwrap_or("");
                if id.is_empty() || linked.is_empty() { continue; }
                let _ = db.execute(
                    "INSERT INTO savings_pockets(id,name,target_amount,current_amount,linked_account_id,target_date,color_tag,is_locked) VALUES(?1,?2,?3,?4,?5,NULL,'#10b981',0) ON CONFLICT(id) DO UPDATE SET name=excluded.name,target_amount=excluded.target_amount,current_amount=excluded.current_amount,linked_account_id=excluded.linked_account_id",
                    rusqlite::params![id, item.get("name").and_then(|v| v.as_str()).unwrap_or(""), item.get("target_amount").map(sheet_int).unwrap_or(0), item.get("current_amount").map(sheet_int).unwrap_or(0), linked],
                );
            }
        }
    }

    // If the sheet returns { transactions: [...] }, merge into local DB (upsert by id).
    if let Some(arr) = payload.get("transactions").and_then(|v| v.as_array()) {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        for item in arr {
            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() {
                continue;
            }
            let exists: bool = db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM transactions WHERE id=?1)",
                    rusqlite::params![id],
                    |r| r.get(0),
                )
                .unwrap_or(false);
            if exists {
                continue;
            }
            let tx = serde_json::json!({
                "id": id,
                "account_id": item.get("account_id").and_then(|v| v.as_str()).unwrap_or(""),
                "destination_account_id": item.get("destination_account_id").and_then(|v| v.as_str()),
                "category_id": item.get("category_id").and_then(|v| v.as_str()).unwrap_or(""),
                "amount": item.get("amount").and_then(|v| v.as_i64()).unwrap_or(0),
                "type": item.get("type").and_then(|v| v.as_str()).unwrap_or("expense"),
                "date": item.get("date").and_then(|v| v.as_i64()).unwrap_or(0),
                "note": item.get("note").and_then(|v| v.as_str()).unwrap_or(""),
            });
            if tx["account_id"].as_str().unwrap_or("").is_empty()
                || tx["category_id"].as_str().unwrap_or("").is_empty()
            {
                continue;
            }
            let _ = db.execute(
                "INSERT OR IGNORE INTO transactions(id,account_id,destination_account_id,category_id,amount,transaction_type,date,note,sync_status) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,'synced')",
                rusqlite::params![tx["id"].as_str(), tx["account_id"].as_str(), tx["destination_account_id"].as_str().unwrap_or(""), tx["category_id"].as_str(), tx["amount"].as_i64().unwrap_or(0), tx["type"].as_str().unwrap_or("expense"), tx["date"].as_i64().unwrap_or(0), tx["note"].as_str().unwrap_or("")],
            );
        }
    }

    Ok(data)
}

pub async fn test_connection(sheet_url: &str) -> Result<String, String> {
    if sheet_url.is_empty() {
        return Err("URL kosong".into());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(sheet_url)
        .send()
        .await
        .map_err(|e| format!("Koneksi gagal: {e}"))?;
    if resp.status().is_success() {
        Ok(format!(
            "Koneksi OK ({}). Sheet dapat dijangkau.",
            resp.status()
        ))
    } else {
        Err(format!(
            "Sheet merespons {} – periksa URL / izin akses.",
            resp.status()
        ))
    }
}
