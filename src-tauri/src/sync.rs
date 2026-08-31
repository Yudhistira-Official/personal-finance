use crate::db::{transactions_all, AppState};
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
    // Collect pending data while holding the lock, then drop the lock before awaiting.
    let payload: Vec<serde_json::Value> =
        {
            let db = state.db.lock().map_err(|e| e.to_string())?;
            let txs = transactions_all(&db, None, None, None, None, None, None, None, None)
                .unwrap_or_default();
            txs.iter()
            .filter(|t| t.sync_status != crate::models::SyncStatus::Synced)
            .map(|t| serde_json::json!({
                "id": t.id, "date": t.date, "type": t.transaction_type.as_str(),
                "account_id": t.account_id, "destination_account_id": t.destination_account_id,
                "category_id": t.category_id, "amount": t.amount, "note": t.note,
                "sync_status": t.sync_status.as_str()
            }))
            .collect()
        };

    if payload.is_empty() {
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
    Ok(SyncResult {
        success: true,
        message: format!("{count} transaksi berhasil dikirim"),
        pushed: count,
    })
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

    // If the sheet returns { transactions: [...] }, merge into local DB (upsert by id).
    if let Some(arr) = data.get("transactions").and_then(|v| v.as_array()) {
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
