use std::collections::{HashMap, HashSet};

use tauri::{Manager, State};

use crate::db::{self, AppState};
use crate::models::{
    Account, AccountInput, AccountType, Category, CategoryInput, CategorySpend, DashboardSummary,
    InvestmentTransaction, MutualFundProduct, Obligation, ObligationInput, ObligationPayment,
    ObligationSummary, PortfolioHolding, PortfolioSnapshot, PortfolioSummary, SavingsPocket,
    SavingsPocketInput, SyncInfo, SyncStatus, Transaction, TransactionInput, TxType,
};
use crate::sync;

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

#[tauri::command]
pub fn accounts_list(state: State<AppState>) -> Result<Vec<Account>, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::accounts_all(&c)
}

#[tauri::command]
pub fn accounts_create(state: State<AppState>, input: AccountInput) -> Result<Account, String> {
    let n = input.name.trim();
    if n.is_empty() {
        return Err("Nama akun wajib diisi".into());
    }
    let now = now_ts();
    let a = Account {
        id: uuid::Uuid::new_v4().to_string(),
        name: n.to_string(),
        account_type: AccountType::from_str(&input.account_type),
        account_number: input.account_number.filter(|s| !s.trim().is_empty()),
        current_balance: input.current_balance,
        is_active: input.is_active.unwrap_or(true),
        created_at: now,
        updated_at: now,
    };
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::accounts_insert(&c, &a)?;
    Ok(a)
}

#[tauri::command]
pub fn accounts_update(
    state: State<AppState>,
    id: String,
    input: AccountInput,
) -> Result<Account, String> {
    let n = input.name.trim();
    if n.is_empty() {
        return Err("Nama akun wajib diisi".into());
    }
    let c = state.db.lock().map_err(|e| e.to_string())?;
    let list = db::accounts_all(&c)?;
    let mut a = list
        .into_iter()
        .find(|a| a.id == id)
        .ok_or("Akun tidak ditemukan")?;
    a.name = n.to_string();
    a.account_type = AccountType::from_str(&input.account_type);
    a.account_number = input.account_number.filter(|s| !s.trim().is_empty());
    a.current_balance = input.current_balance;
    if let Some(v) = input.is_active {
        a.is_active = v;
    }
    a.updated_at = now_ts();
    db::accounts_update(&c, &a)?;
    Ok(a)
}

#[tauri::command]
pub fn accounts_delete(state: State<AppState>, id: String) -> Result<(), String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::accounts_delete(&c, &id)
}

#[tauri::command]
pub fn categories_list(state: State<AppState>) -> Result<Vec<Category>, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::categories_all(&c)
}

#[tauri::command]
pub fn categories_create(state: State<AppState>, input: CategoryInput) -> Result<Category, String> {
    let n = input.name.trim();
    if n.is_empty() {
        return Err("Nama kategori wajib diisi".into());
    }
    let cat = Category {
        id: uuid::Uuid::new_v4().to_string(),
        name: n.to_string(),
        category_type: TxType::from_str(&input.category_type),
        icon: if input.icon.trim().is_empty() {
            "wallet".into()
        } else {
            input.icon
        },
        color_hex: if input.color_hex.trim().is_empty() {
            "#6366f1".into()
        } else {
            input.color_hex
        },
    };
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::categories_insert(&c, &cat)?;
    Ok(cat)
}

#[tauri::command]
pub fn categories_delete(state: State<AppState>, id: String) -> Result<(), String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::categories_delete(&c, &id)
}

#[allow(clippy::too_many_arguments)]
#[tauri::command(rename_all = "snake_case")]
pub fn transactions_list(
    state: State<AppState>,
    limit: Option<i64>,
    offset: Option<i64>,
    search: Option<String>,
    account_id: Option<String>,
    category_id: Option<String>,
    tx_type: Option<String>,
    from: Option<i64>,
    to: Option<i64>,
) -> Result<Vec<Transaction>, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::transactions_all(
        &c,
        limit,
        offset,
        search,
        account_id,
        category_id,
        tx_type,
        from,
        to,
    )
}

#[tauri::command]
pub fn transactions_create(
    state: State<AppState>,
    input: TransactionInput,
) -> Result<Transaction, String> {
    if input.amount <= 0 {
        return Err("Nominal harus > 0".into());
    }
    if input.account_id.trim().is_empty() {
        return Err("Akun wajib dipilih".into());
    }
    if input.category_id.trim().is_empty() {
        return Err("Kategori wajib dipilih".into());
    }
    let tt = TxType::from_str(&input.transaction_type);
    let c = state.db.lock().map_err(|e| e.to_string())?;
    if tt == TxType::Transfer {
        if input
            .destination_account_id
            .as_deref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            return Err("Akun tujuan wajib untuk transfer".into());
        }
        if input.destination_account_id.as_deref() == Some(&input.account_id) {
            return Err("Akun tujuan tidak boleh sama dengan akun sumber".into());
        }
    }
    let tx = Transaction {
        id: uuid::Uuid::new_v4().to_string(),
        account_id: input.account_id,
        destination_account_id: input
            .destination_account_id
            .filter(|s| !s.trim().is_empty()),
        category_id: input.category_id,
        amount: input.amount,
        transaction_type: tt,
        date: if input.date == 0 {
            now_ts()
        } else {
            input.date
        },
        note: input.note,
        receipt_url: input.receipt_url.filter(|s| !s.trim().is_empty()),
        sync_status: SyncStatus::Pending,
        sheet_row_id: None,
    };
    db::transactions_insert(&c, &tx)?;
    let delta = match tt {
        TxType::Income => tx.amount,
        TxType::Expense => -tx.amount,
        TxType::Transfer => -tx.amount,
    };
    c.execute(
        "UPDATE accounts SET current_balance=current_balance+?1,updated_at=?2 WHERE id=?3",
        rusqlite::params![delta, now_ts(), tx.account_id],
    )
    .map_err(|e| e.to_string())?;
    if tt == TxType::Transfer {
        if let Some(ref dest) = tx.destination_account_id {
            c.execute(
                "UPDATE accounts SET current_balance=current_balance+?1,updated_at=?2 WHERE id=?3",
                rusqlite::params![tx.amount, now_ts(), dest],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(tx)
}

#[tauri::command]
pub fn transactions_update(
    state: State<AppState>,
    id: String,
    input: TransactionInput,
) -> Result<Transaction, String> {
    if input.amount <= 0 {
        return Err("Nominal harus > 0".into());
    }
    if input.account_id.trim().is_empty() {
        return Err("Akun wajib dipilih".into());
    }
    if input.category_id.trim().is_empty() {
        return Err("Kategori wajib dipilih".into());
    }
    let new_type = TxType::from_str(&input.transaction_type);
    if new_type == TxType::Transfer {
        if input
            .destination_account_id
            .as_deref()
            .map(|s| s.trim().is_empty())
            .unwrap_or(true)
        {
            return Err("Akun tujuan wajib untuk transfer".into());
        }
        if input.destination_account_id.as_deref() == Some(&input.account_id) {
            return Err("Akun tujuan tidak boleh sama dengan akun sumber".into());
        }
    }
    let c = state.db.lock().map_err(|e| e.to_string())?;
    let list = db::transactions_all(&c, None, None, None, None, None, None, None, None)?;
    let mut old = list
        .into_iter()
        .find(|t| t.id == id)
        .ok_or("Transaksi tidak ditemukan")?;
    let old_type = old.transaction_type;
    let old_amt = old.amount;
    let old_src = old.account_id.clone();
    let old_dst = old.destination_account_id.clone();
    let revert_src = match old_type {
        TxType::Income => -old_amt,
        _ => old_amt,
    };
    c.execute(
        "UPDATE accounts SET current_balance=current_balance+?1,updated_at=?2 WHERE id=?3",
        rusqlite::params![revert_src, now_ts(), old_src],
    )
    .map_err(|e| e.to_string())?;
    if old_type == TxType::Transfer {
        if let Some(ref d) = old_dst {
            c.execute(
                "UPDATE accounts SET current_balance=current_balance-?1,updated_at=?2 WHERE id=?3",
                rusqlite::params![old_amt, now_ts(), d],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    old.account_id = input.account_id;
    old.destination_account_id = input
        .destination_account_id
        .filter(|s| !s.trim().is_empty());
    old.category_id = input.category_id;
    old.amount = input.amount;
    old.transaction_type = new_type;
    old.date = if input.date == 0 {
        now_ts()
    } else {
        input.date
    };
    old.note = input.note;
    old.receipt_url = input.receipt_url.filter(|s| !s.trim().is_empty());
    old.sync_status = SyncStatus::Pending;
    db::transactions_update(&c, &old)?;
    let delta = match old.transaction_type {
        TxType::Income => old.amount,
        _ => -old.amount,
    };
    c.execute(
        "UPDATE accounts SET current_balance=current_balance+?1,updated_at=?2 WHERE id=?3",
        rusqlite::params![delta, now_ts(), old.account_id],
    )
    .map_err(|e| e.to_string())?;
    if old.transaction_type == TxType::Transfer {
        if let Some(ref d) = old.destination_account_id {
            c.execute(
                "UPDATE accounts SET current_balance=current_balance+?1,updated_at=?2 WHERE id=?3",
                rusqlite::params![old.amount, now_ts(), d],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(old)
}

#[tauri::command]
pub fn transactions_delete(state: State<AppState>, id: String) -> Result<(), String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    let list = db::transactions_all(&c, None, None, None, None, None, None, None, None)?;
    if let Some(t) = list.into_iter().find(|t| t.id == id) {
        let rev = match t.transaction_type {
            TxType::Income => -t.amount,
            _ => t.amount,
        };
        c.execute(
            "UPDATE accounts SET current_balance=current_balance+?1,updated_at=?2 WHERE id=?3",
            rusqlite::params![rev, now_ts(), t.account_id],
        )
        .map_err(|e| e.to_string())?;
        if t.transaction_type == TxType::Transfer {
            if let Some(d) = t.destination_account_id {
                c.execute("UPDATE accounts SET current_balance=current_balance-?1,updated_at=?2 WHERE id=?3",
                    rusqlite::params![t.amount, now_ts(), d]).map_err(|e| e.to_string())?;
            }
        }
    }
    db::transactions_delete(&c, &id)
}

#[tauri::command]
pub fn pockets_list(state: State<AppState>) -> Result<Vec<SavingsPocket>, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::pockets_all(&c)
}

#[tauri::command]
pub fn pockets_create(
    state: State<AppState>,
    input: SavingsPocketInput,
) -> Result<SavingsPocket, String> {
    let n = input.name.trim();
    if n.is_empty() {
        return Err("Nama kantong wajib diisi".into());
    }
    if input.target_amount <= 0 {
        return Err("Target nominal harus > 0".into());
    }
    let c = state.db.lock().map_err(|e| e.to_string())?;
    let p = SavingsPocket {
        id: uuid::Uuid::new_v4().to_string(),
        name: n.to_string(),
        target_amount: input.target_amount,
        current_amount: input.current_amount.unwrap_or(0),
        linked_account_id: input.linked_account_id,
        target_date: input.target_date,
        color_tag: if input.color_tag.trim().is_empty() {
            "#10b981".into()
        } else {
            input.color_tag
        },
        is_locked: input.is_locked.unwrap_or(false),
    };
    // Initial pocket funds are transferred out of its linked account.
    if p.current_amount != 0 {
        c.execute(
            "UPDATE accounts SET current_balance=current_balance-?1,updated_at=?2 WHERE id=?3",
            rusqlite::params![p.current_amount, now_ts(), p.linked_account_id],
        )
        .map_err(|e| e.to_string())?;
    }
    db::pockets_insert(&c, &p)?;
    Ok(p)
}

#[tauri::command]
pub fn pockets_update(
    state: State<AppState>,
    id: String,
    input: SavingsPocketInput,
) -> Result<SavingsPocket, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    let mut p = db::pockets_one(&c, &id)?.ok_or("Kantong tidak ditemukan")?;
    let n = input.name.trim();
    if n.is_empty() {
        return Err("Nama kantong wajib diisi".into());
    }
    let old_amount = p.current_amount;
    let old_account = p.linked_account_id.clone();
    p.name = n.to_string();
    p.target_amount = input.target_amount;
    if let Some(v) = input.current_amount {
        p.current_amount = v;
    }
    p.linked_account_id = input.linked_account_id;
    p.target_date = input.target_date;
    p.color_tag = if input.color_tag.trim().is_empty() {
        "#10b981".into()
    } else {
        input.color_tag
    };
    if let Some(v) = input.is_locked {
        p.is_locked = v;
    }
    let delta = p.current_amount - old_amount;
    if p.linked_account_id == old_account {
        // Funds moved into the pocket reduce the account; withdrawn funds return to it.
        if delta != 0 {
            c.execute(
                "UPDATE accounts SET current_balance=current_balance-?1,updated_at=?2 WHERE id=?3",
                rusqlite::params![delta, now_ts(), p.linked_account_id],
            )
            .map_err(|e| e.to_string())?;
        }
    } else {
        // Pocket reassigned to a new account: return old funds, deduct new funds.
        if old_amount != 0 {
            c.execute(
                "UPDATE accounts SET current_balance=current_balance+?1,updated_at=?2 WHERE id=?3",
                rusqlite::params![old_amount, now_ts(), old_account],
            )
            .map_err(|e| e.to_string())?;
        }
        if p.current_amount != 0 {
            c.execute(
                "UPDATE accounts SET current_balance=current_balance-?1,updated_at=?2 WHERE id=?3",
                rusqlite::params![p.current_amount, now_ts(), p.linked_account_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    db::pockets_update(&c, &p)?;
    Ok(p)
}

#[tauri::command]
pub fn pockets_delete(state: State<AppState>, id: String) -> Result<(), String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::pockets_delete(&c, &id)
}

#[tauri::command]
pub fn pockets_deposit(
    state: State<AppState>,
    id: String,
    amount: i64,
) -> Result<SavingsPocket, String> {
    if amount <= 0 {
        return Err("Nominal setor harus > 0".into());
    }
    let c = state.db.lock().map_err(|e| e.to_string())?;
    let mut p = db::pockets_one(&c, &id)?.ok_or("Kantong tidak ditemukan")?;
    if p.is_locked {
        return Err("Kantong terkunci".into());
    }
    p.current_amount += amount;
    c.execute(
        "UPDATE accounts SET current_balance=current_balance-?1,updated_at=?2 WHERE id=?3",
        rusqlite::params![amount, now_ts(), p.linked_account_id],
    )
    .map_err(|e| e.to_string())?;
    db::pockets_update(&c, &p)?;
    Ok(p)
}

#[tauri::command]
pub fn pockets_withdraw(
    state: State<AppState>,
    id: String,
    amount: i64,
) -> Result<SavingsPocket, String> {
    if amount <= 0 {
        return Err("Nominal tarik harus > 0".into());
    }
    let c = state.db.lock().map_err(|e| e.to_string())?;
    let mut p = db::pockets_one(&c, &id)?.ok_or("Kantong tidak ditemukan")?;
    if amount > p.current_amount {
        return Err("Saldo kantong tidak cukup".into());
    }
    if p.is_locked {
        return Err("Kantong terkunci".into());
    }
    p.current_amount -= amount;
    c.execute(
        "UPDATE accounts SET current_balance=current_balance+?1,updated_at=?2 WHERE id=?3",
        rusqlite::params![amount, now_ts(), p.linked_account_id],
    )
    .map_err(|e| e.to_string())?;
    db::pockets_update(&c, &p)?;
    Ok(p)
}

#[tauri::command]
pub fn dashboard_summary(
    state: State<AppState>,
    from: i64,
    to: i64,
) -> Result<DashboardSummary, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::dashboard(&c, from, to)
}

#[tauri::command]
pub fn expense_by_category(
    state: State<AppState>,
    from: i64,
    to: i64,
) -> Result<Vec<CategorySpend>, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::expense_by_category(&c, from, to)
}

#[tauri::command]
pub fn export_csv(
    state: State<AppState>,
    from: Option<i64>,
    to: Option<i64>,
) -> Result<String, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::export_transactions_csv(&c, from, to)
}

#[tauri::command]
pub fn sync_status(state: State<AppState>) -> Result<SyncInfo, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    let pending = db::pending_count(&c);
    let url = state.sheet_url.lock().map_err(|e| e.to_string())?.clone();
    let status = if pending == 0 {
        "synced".to_string()
    } else {
        "pending".to_string()
    };
    let auto = *state.auto_sync.lock().map_err(|e| e.to_string())?;
    Ok(SyncInfo {
        status,
        pending_count: pending,
        sheet_url: url,
        auto_sync: auto,
    })
}

#[tauri::command]
pub async fn sync_test(state: State<'_, AppState>, url: String) -> Result<String, String> {
    let target = if url.trim().is_empty() {
        state
            .sheet_url
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .unwrap_or_default()
    } else {
        url
    };
    sync::test_connection(&target).await
}

#[tauri::command]
pub async fn sync_push(state: State<'_, AppState>) -> Result<sync::SyncResult, String> {
    let url = state
        .sheet_url
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .unwrap_or_default();
    let result = sync::push_to_sheet(&url, &state).await?;
    if result.success {
        let c = state.db.lock().map_err(|e| e.to_string())?;
        c.execute("UPDATE transactions SET sync_status='synced' WHERE sync_status='pending' OR sync_status='failed'",
            []).map_err(|e| e.to_string())?;
    }
    Ok(result)
}

#[tauri::command]
pub async fn sync_fetch(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let url = state
        .sheet_url
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .unwrap_or_default();
    sync::fetch_from_sheet(&url, &state).await
}

#[tauri::command(rename_all = "snake_case")]
pub fn settings_save(
    state: State<AppState>,
    sheet_url: Option<String>,
    auto_sync: bool,
) -> Result<(), String> {
    let url = sheet_url
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    {
        let mut g = state.sheet_url.lock().map_err(|e| e.to_string())?;
        *g = url.clone();
    }
    {
        let mut g = state.auto_sync.lock().map_err(|e| e.to_string())?;
        *g = auto_sync;
    }
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::save_settings(&c, &url, auto_sync)
}

/// Fetches Bibit's catalogue without holding SQLite's mutex across the network await,
/// then replaces each cached product snapshot and returns the number processed.
#[tauri::command]
pub async fn sync_bibit_catalog(state: State<'_, AppState>) -> Result<u32, String> {
    // Network I/O happens before locking so other IPC commands remain responsive.
    let products = crate::bibit::BibitClient::new().fetch_catalog().await?;
    let mut c = state.db.lock().map_err(|e| e.to_string())?;
    // Bungkus batch upsert dalam satu transaksi: atomik (gagal di tengah tidak
    // meninggalkan cache sebagian) dan jauh lebih cepat daripada ~3000 autocommit.
    // Trigger AFTER INSERT/UPDATE tetap menjaga index FTS per baris, jadi tidak perlu
    // rebuild manual setelah loop.
    let tx = c.transaction().map_err(|e| e.to_string())?;
    for product in &products {
        db::bibit_product_upsert(&*tx, product)?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(products.len() as u32)
}

/// Searches locally cached mutual-fund products by name/manager and optional fund type.
///
/// Fallback: if the local cache is empty and query non-empty, try Bibit's server-side
/// search (`name=...`). This handles fresh installs and stale/sync-failed caches.
/// The DB lock is never held across `await`; remote fetch happens first, then a
/// short transaction caches the results so subsequent local searches hit FTS.
#[tauri::command(rename_all = "snake_case")]
pub async fn search_mutual_funds(
    state: State<'_, AppState>,
    query: String,
    fund_type: Option<String>,
) -> Result<Vec<MutualFundProduct>, String> {
    let local_results = {
        let c = state.db.lock().map_err(|e| e.to_string())?;
        db::bibit_product_search(&c, &query, fund_type.as_deref())?
    };
    if !local_results.is_empty() || query.trim().is_empty() {
        return Ok(local_results);
    }

    // Remote search runs outside the SQLite lock to keep IPC responsive.
    // Offline/remote failure: fall back to local results (empty in this branch)
    // instead of propagating an error, so the UI shows no results without an error.
    let remote_results = crate::bibit::BibitClient::new()
        .search_remote(query.trim())
        .await
        .unwrap_or(local_results);

    // Cache raw remote hits so later local searches (any fund_type) can reuse them.
    if !remote_results.is_empty() {
        let mut c = state.db.lock().map_err(|e| e.to_string())?;
        let tx = c.transaction().map_err(|e| e.to_string())?;
        for product in &remote_results {
            db::bibit_product_upsert(&*tx, product)?;
        }
        tx.commit().map_err(|e| e.to_string())?;
    }

    // The remote API ignores fund_type; filter locally before returning.
    let mut results = remote_results;
    if let Some(expected) = fund_type.as_deref() {
        results.retain(|p| p.fund_type == expected);
    }
    Ok(results)
}

/// Validates, stores, and atomically applies one investment transaction to its cash account.
#[tauri::command]
pub fn record_investment_tx(
    state: State<AppState>,
    payload: InvestmentTransaction,
) -> Result<InvestmentTransaction, String> {
    // Reject unknown types first so DIVIDEND-specific rules below are unambiguous.
    if !matches!(payload.tx_type.as_str(), "BUY" | "SELL" | "DIVIDEND") {
        return Err("Tipe transaksi investasi tidak valid".into());
    }
    let is_dividend = payload.tx_type == "DIVIDEND";
    // Cash dividends may carry zero units/NAV; only BUY/SELL must trade real units.
    if !payload.units.is_finite() || (!is_dividend && payload.units <= 0.0) || payload.units < 0.0 {
        return Err(if is_dividend {
            "Units dividen tidak boleh negatif".into()
        } else {
            "Units harus > 0".into()
        });
    }
    if !payload.nav_per_unit.is_finite()
        || (!is_dividend && payload.nav_per_unit <= 0.0)
        || payload.nav_per_unit < 0.0
    {
        return Err(if is_dividend {
            "NAV dividen tidak boleh negatif".into()
        } else {
            "NAV harus > 0".into()
        });
    }
    if payload.fee < 0 {
        return Err("Fee tidak boleh negatif".into());
    }

    let c = state.db.lock().map_err(|e| e.to_string())?;
    c.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| e.to_string())?;
    let result = (|| {
        // Anti-oversell: SELL cannot exceed units owned (BUY minus prior SELL, f64).
        if payload.tx_type == "SELL" {
            let buy_units: f64 = c
                .query_row(
                    "SELECT COALESCE(SUM(units),0) FROM investment_transactions WHERE product_id=?1 AND tx_type='BUY'",
                    rusqlite::params![payload.product_id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            let sell_units: f64 = c
                .query_row(
                    "SELECT COALESCE(SUM(units),0) FROM investment_transactions WHERE product_id=?1 AND tx_type='SELL'",
                    rusqlite::params![payload.product_id],
                    |r| r.get(0),
                )
                .map_err(|e| e.to_string())?;
            let owned = buy_units - sell_units;
            if payload.units > owned + 1e-9 {
                return Err(format!(
                    "Unit jual melebihi kepemilikan ({owned:.4} tersedia)"
                ));
            }
        }

        db::investment_tx_insert(&c, &payload)?;
        // BUY pays total+fee; SELL receives total-fee; DIVIDEND receives total.
        // checked_* prevents i64 overflow from maliciously huge IPC payloads.
        let delta = match payload.tx_type.as_str() {
            "BUY" => payload
                .total_amount
                .checked_add(payload.fee)
                .map(|v| -v)
                .ok_or("Nominal total + fee overflow")?,
            "SELL" => payload.total_amount.saturating_sub(payload.fee).max(0),
            _ => payload.total_amount,
        };
        c.execute(
            "UPDATE accounts SET current_balance=current_balance+?1,updated_at=?2 WHERE id=?3",
            rusqlite::params![delta, now_ts(), payload.account_id],
        )
        .map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    })();
    match result {
        Ok(()) => {
            c.execute_batch("COMMIT").map_err(|e| e.to_string())?;
            Ok(payload)
        }
        Err(error) => {
            let _ = c.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

/// Aggregates investment transactions into current holdings using cached NAV values.
///
/// Cost basis uses the average-cost method: a SELL removes cost proportionally
/// (sold_units / units_before * cost_before), not the sale proceeds — realized
/// gain must stay out of the remaining invested amount.
pub(crate) fn compute_holdings(
    conn: &rusqlite::Connection,
) -> Result<Vec<PortfolioHolding>, String> {
    let transactions = db::investment_tx_all(conn)?;
    let products = db::bibit_products_all(conn)?;
    let product_map: HashMap<String, MutualFundProduct> = products
        .into_iter()
        .map(|product| (product.id.clone(), product))
        .collect();
    // Ledger per product: (units held, remaining cost basis) in f64 until final rounding.
    let mut totals: HashMap<String, (f64, f64)> = HashMap::new();

    // Replay chronologically (oldest first — investment_tx_all returns DESC).
    for transaction in transactions.iter().rev() {
        let entry = totals
            .entry(transaction.product_id.clone())
            .or_insert((0.0, 0.0));
        match transaction.tx_type.as_str() {
            "BUY" => {
                entry.0 += transaction.units;
                entry.1 += transaction.total_amount as f64;
            }
            "SELL" => {
                if entry.0 > 0.0 {
                    let sold = transaction.units.min(entry.0);
                    // Remove proportional cost, then clamp so cost reaches 0 on full sell.
                    entry.1 -= sold / entry.0 * entry.1;
                    entry.0 -= sold;
                    if entry.0 <= 1e-9 {
                        entry.0 = 0.0;
                        entry.1 = 0.0;
                    }
                }
            }
            // DIVIDEND is cash-only: it changes neither units nor cost basis.
            _ => {}
        }
    }

    let mut holdings = Vec::new();
    for (product_id, (total_units, cost_total)) in totals {
        // Guard: no units left means nothing to report, whatever the cost residue is.
        if total_units <= 0.0 {
            continue;
        }
        let total_invested = cost_total.max(0.0).round() as i64;
        let product = product_map.get(&product_id);
        let current_nav = product.map(|p| p.current_nav).unwrap_or(0.0);
        let current_value = (total_units * current_nav).round() as i64;
        let unrealized_pnl = current_value - total_invested;
        holdings.push(PortfolioHolding {
            product_id,
            product_name: product.map(|p| p.name.clone()).unwrap_or_default(),
            fund_type: product.map(|p| p.fund_type.clone()).unwrap_or_default(),
            manager_name: product.map(|p| p.manager_name.clone()).unwrap_or_default(),
            total_units,
            avg_buy_nav: if total_units > 0.0 {
                cost_total / total_units
            } else {
                0.0
            },
            total_invested,
            current_nav,
            current_value,
            unrealized_pnl,
            roi_percentage: if total_invested > 0 {
                unrealized_pnl as f64 / total_invested as f64 * 100.0
            } else {
                0.0
            },
        });
    }
    Ok(holdings)
}

#[tauri::command]
pub fn get_portfolio_holdings(state: State<AppState>) -> Result<Vec<PortfolioHolding>, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    compute_holdings(&c)
}

/// Records today's portfolio totals as one daily snapshot row (INSERT OR REPLACE per day).
#[tauri::command]
pub fn record_daily_snapshot(state: State<AppState>) -> Result<PortfolioSnapshot, String> {
    record_daily_snapshot_from(&state)
}

/// Records today's portfolio snapshot without a Tauri `State` wrapper (background job path).
fn record_daily_snapshot_from(state: &AppState) -> Result<PortfolioSnapshot, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    let holdings = compute_holdings(&c)?;
    let total_value = holdings.iter().map(|h| h.current_value).sum::<i64>();
    let total_invested = holdings.iter().map(|h| h.total_invested).sum::<i64>();
    let unrealized_pnl = holdings.iter().map(|h| h.unrealized_pnl).sum::<i64>();
    drop(c);
    let day = chrono::Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(chrono::Local)
        .unwrap()
        .timestamp();
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::snapshot_upsert(&c, day, total_value, total_invested, unrealized_pnl)?;
    Ok(PortfolioSnapshot {
        day,
        total_value,
        total_invested,
        unrealized_pnl,
    })
}
#[tauri::command]
pub fn get_portfolio_snapshots(
    state: State<AppState>,
    days: Option<i64>,
) -> Result<Vec<PortfolioSnapshot>, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::snapshots_last(&c, days.unwrap_or(30))
}

/// Refreshes NAV for owned products only via per-product name-fragment searches,
/// avoiding the full catalogue pagination that made the refresh feel stuck.
/// (Bibit's search API matches by name, not product ID, so the query is derived
/// from the cached product name; unmatched products keep their stale NAV.)
async fn refresh_navs(state: &AppState) -> Result<u32, String> {
    let (product_ids, held_names) = {
        let c = state.db.lock().map_err(|e| e.to_string())?;
        let transactions = db::investment_tx_all(&c)?;
        let product_ids: HashSet<String> = transactions
            .into_iter()
            .map(|transaction| transaction.product_id)
            .collect();
        let products = db::bibit_products_all(&c)?;
        let product_map: HashMap<String, String> = products
            .into_iter()
            .map(|product| (product.id, product.name))
            .collect();
        let held_names: HashMap<String, String> = product_ids
            .iter()
            .filter_map(|id| product_map.get(id).map(|name| (id.clone(), name.clone())))
            .collect();
        (product_ids, held_names)
    };
    if product_ids.is_empty() {
        return Ok(0);
    }

    let navs = crate::bibit::BibitClient::new()
        .fetch_nav_batch(&held_names)
        .await?;
    let now = now_ts();
    let c = state.db.lock().map_err(|e| e.to_string())?;
    let mut refreshed = 0;
    for (product_id, current_nav) in navs {
        // Skip missing products so a failed lookup never overwrites cached NAV.
        c.execute(
            "UPDATE bibit_products_cache SET current_nav=?1,last_fetched_at=?2 WHERE id=?3",
            rusqlite::params![current_nav, now, product_id],
        )
        .map_err(|e| e.to_string())?;
        refreshed += 1;
    }
    Ok(refreshed)
}

#[tauri::command]
pub async fn refresh_portfolio_nav(state: State<'_, AppState>) -> Result<u32, String> {
    refresh_navs(&state).await
}

fn obligation_from_input(
    input: ObligationInput,
    existing: Option<Obligation>,
) -> Result<Obligation, String> {
    let counterparty = input.counterparty.trim();
    let title = input.title.trim();
    if counterparty.is_empty() || title.is_empty() {
        return Err("Counterparty dan judul wajib diisi".into());
    }
    if !matches!(input.direction.as_str(), "DEBT" | "RECEIVABLE") {
        return Err("Arah obligasi tidak valid".into());
    }
    if input.original_amount <= 0 {
        return Err("Nominal asli harus > 0".into());
    }
    let remaining = input
        .remaining_amount
        .unwrap_or(input.original_amount)
        .clamp(0, input.original_amount);
    let now = now_ts();
    let old = existing.unwrap_or_else(|| Obligation {
        id: uuid::Uuid::new_v4().to_string(),
        direction: String::new(),
        counterparty: String::new(),
        title: String::new(),
        original_amount: 0,
        remaining_amount: 0,
        due_date: None,
        note: None,
        status: String::new(),
        created_at: now,
        updated_at: now,
    });
    Ok(Obligation {
        id: old.id,
        direction: input.direction,
        counterparty: counterparty.to_string(),
        title: title.to_string(),
        original_amount: input.original_amount,
        remaining_amount: remaining,
        due_date: input.due_date,
        note: input.note.filter(|v| !v.trim().is_empty()),
        status: if remaining == 0 {
            "DONE".into()
        } else {
            "OPEN".into()
        },
        created_at: old.created_at,
        updated_at: now,
    })
}

#[tauri::command]
pub fn obligations_list(state: State<AppState>) -> Result<Vec<Obligation>, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::obligations_all(&c)
}

#[tauri::command]
pub fn obligations_summary(state: State<AppState>) -> Result<ObligationSummary, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::obligation_summary(&c)
}

#[tauri::command]
pub fn obligation_create(
    state: State<AppState>,
    input: ObligationInput,
) -> Result<Obligation, String> {
    let o = obligation_from_input(input, None)?;
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::obligation_insert(&c, &o)?;
    Ok(o)
}

#[tauri::command]
pub fn obligation_update(
    state: State<AppState>,
    id: String,
    input: ObligationInput,
) -> Result<Obligation, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    let old = db::obligation_one(&c, &id)?.ok_or("Obligasi tidak ditemukan")?;
    let o = obligation_from_input(input, Some(old))?;
    db::obligation_update(&c, &o)?;
    Ok(o)
}

#[tauri::command]
pub fn obligation_delete(state: State<AppState>, id: String) -> Result<(), String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::obligation_delete(&c, &id)
}

#[tauri::command]
pub fn obligation_pay(
    state: State<AppState>,
    payment: ObligationPayment,
) -> Result<Obligation, String> {
    if payment.amount <= 0 {
        return Err("Nominal pembayaran harus > 0".into());
    }
    let c = state.db.lock().map_err(|e| e.to_string())?;
    c.execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| e.to_string())?;
    let result = (|| {
        let mut o =
            db::obligation_one(&c, &payment.obligation_id)?.ok_or("Obligasi tidak ditemukan")?;
        if payment.amount > o.remaining_amount {
            return Err("Pembayaran melebihi sisa obligasi".into());
        }
        let remaining = o.remaining_amount - payment.amount;
        let date = payment.date.unwrap_or_else(now_ts);
        if let Some(account_id) = payment.account_id {
            let tx_type = if o.direction == "DEBT" {
                TxType::Expense
            } else {
                TxType::Income
            };
            let category_type = tx_type.as_str();
            let category_id: String = c
                .query_row(
                    "SELECT id FROM categories WHERE category_type=?1 ORDER BY name LIMIT 1",
                    rusqlite::params![category_type],
                    |r| r.get(0),
                )
                .map_err(|_| "Kategori transaksi tidak tersedia".to_string())?;
            let tx = Transaction {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: account_id.clone(),
                destination_account_id: None,
                category_id,
                amount: payment.amount,
                transaction_type: tx_type,
                date,
                note: format!("Pembayaran {}: {}", o.direction, o.title),
                receipt_url: None,
                sync_status: SyncStatus::Pending,
                sheet_row_id: None,
            };
            db::transactions_insert(&c, &tx)?;
            let delta = if tx_type == TxType::Expense {
                -payment.amount
            } else {
                payment.amount
            };
            if c.execute(
                "UPDATE accounts SET current_balance=current_balance+?1,updated_at=?2 WHERE id=?3",
                rusqlite::params![delta, now_ts(), account_id],
            )
            .map_err(|e| e.to_string())?
                == 0
            {
                return Err("Akun tidak ditemukan".into());
            }
        }
        o.remaining_amount = remaining;
        o.status = if remaining == 0 {
            "DONE".into()
        } else {
            "OPEN".into()
        };
        o.updated_at = now_ts();
        db::obligation_update(&c, &o)?;
        Ok(o)
    })();
    match result {
        Ok(o) => {
            c.execute_batch("COMMIT").map_err(|e| e.to_string())?;
            Ok(o)
        }
        Err(e) => {
            let _ = c.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}

#[tauri::command]
pub fn portfolio_summary(state: State<AppState>) -> Result<PortfolioSummary, String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    let holdings = compute_holdings(&c)?;
    Ok(PortfolioSummary {
        total_value: holdings.iter().map(|h| h.current_value).sum(),
        total_invested: holdings.iter().map(|h| h.total_invested).sum(),
        unrealized_pnl: holdings.iter().map(|h| h.unrealized_pnl).sum(),
    })
}

#[tauri::command]
pub fn reset_data(state: State<AppState>) -> Result<(), String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::reset_all_data(&c)
}

// ── Background job: daily NAV refresh + snapshot ─────────────────────────────

/// Refreshes owned-product NAVs and records today's portfolio snapshot.
/// All failures are swallowed into eprintln — background tasks must never crash the app.
pub async fn daily_portfolio_job(app: tauri::AppHandle) {
    let state = app.state::<AppState>();
    match refresh_navs(&state).await {
        Ok(refreshed) => eprintln!("daily_portfolio_job: refreshed {refreshed} NAV"),
        Err(e) => eprintln!("daily_portfolio_job: refresh NAV failed: {e}"),
    }
    match record_daily_snapshot_from(&state) {
        Ok(s) => eprintln!(
            "daily_portfolio_job: snapshot day={} value={} invested={} pnl={}",
            s.day, s.total_value, s.total_invested, s.unrealized_pnl
        ),
        Err(e) => eprintln!("daily_portfolio_job: snapshot failed: {e}"),
    }
}
