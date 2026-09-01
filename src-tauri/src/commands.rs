use tauri::State;

use crate::db::{self, AppState};
use crate::models::{
    Account, AccountInput, AccountType, Category, CategoryInput, CategorySpend, DashboardSummary,
    SavingsPocket, SavingsPocketInput, SyncInfo, SyncStatus, Transaction, TransactionInput, TxType,
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

#[tauri::command]
pub fn reset_data(state: State<AppState>) -> Result<(), String> {
    let c = state.db.lock().map_err(|e| e.to_string())?;
    db::reset_all_data(&c)
}
