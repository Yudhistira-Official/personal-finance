use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

use crate::models::{
    Account, AccountType, Category, CategorySpend, DashboardSummary, SavingsPocket, SyncStatus,
    Transaction as PfTransaction, TxType,
};

fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
    let data = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir fail: {e}"))?;
    std::fs::create_dir_all(&data).map_err(|e| format!("create data dir: {e}"))?;
    Ok(data.join("personal_finance.db"))
}

pub struct AppState {
    pub db: Mutex<Connection>,
    pub sheet_url: Mutex<Option<String>>,
    pub auto_sync: Mutex<bool>,
}

impl AppState {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let path = db_path(app)?;
        let conn = Connection::open(&path).map_err(|e| format!("DB open {path:?}: {e}"))?;
        init_db(&conn)?;
        let s = load_settings(&conn);
        Ok(Self {
            db: Mutex::new(conn),
            sheet_url: Mutex::new(s.0),
            auto_sync: Mutex::new(s.1),
        })
    }
}

pub fn init_db(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS accounts (
          id TEXT PRIMARY KEY, name TEXT NOT NULL, account_type TEXT NOT NULL,
          account_number TEXT, current_balance INTEGER NOT NULL DEFAULT 0,
          is_active INTEGER NOT NULL DEFAULT 1,
          created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS categories (
          id TEXT PRIMARY KEY, name TEXT NOT NULL, category_type TEXT NOT NULL,
          icon TEXT NOT NULL DEFAULT 'wallet', color_hex TEXT NOT NULL DEFAULT '#6366f1'
        );
        CREATE TABLE IF NOT EXISTS transactions (
          id TEXT PRIMARY KEY, account_id TEXT NOT NULL, destination_account_id TEXT,
          category_id TEXT NOT NULL, amount INTEGER NOT NULL,
          transaction_type TEXT NOT NULL, date INTEGER NOT NULL, note TEXT NOT NULL DEFAULT '',
          receipt_url TEXT, sync_status TEXT NOT NULL DEFAULT 'pending', sheet_row_id INTEGER,
          FOREIGN KEY(account_id) REFERENCES accounts(id), FOREIGN KEY(category_id) REFERENCES categories(id)
        );
        CREATE TABLE IF NOT EXISTS savings_pockets (
          id TEXT PRIMARY KEY, name TEXT NOT NULL, target_amount INTEGER NOT NULL,
          current_amount INTEGER NOT NULL DEFAULT 0, linked_account_id TEXT NOT NULL,
          target_date INTEGER, color_tag TEXT NOT NULL DEFAULT '#10b981', is_locked INTEGER NOT NULL DEFAULT 0,
          FOREIGN KEY(linked_account_id) REFERENCES accounts(id)
        );
        CREATE TABLE IF NOT EXISTS settings (
          key TEXT PRIMARY KEY, value TEXT NOT NULL
        );
        ",
    )
    .map_err(|e| format!("init_db: {e}"))?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| format!("foreign_keys: {e}"))?;

    let cnt: i64 = conn
        .query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))
        .unwrap_or(0);
    if cnt == 0 {
        seed_categories(conn)?;
    }
    Ok(())
}

fn seed_categories(conn: &Connection) -> Result<(), String> {
    let defaults: &[(&str, &str, &str, &str)] = &[
        ("Makanan & Minuman", "expense", "utensils", "#f97316"),
        ("Transportasi", "expense", "bus", "#38bdf8"),
        ("Belanja", "expense", "shopping-bag", "#ec4899"),
        ("Tagihan", "expense", "receipt", "#ef4444"),
        ("Kesehatan", "expense", "heart", "#a78bfa"),
        ("Hiburan", "expense", "film", "#facc15"),
        ("Gaji", "income", "wallet", "#10b981"),
        ("Bonus", "income", "gift", "#22c55e"),
        ("Investasi", "income", "trending-up", "#06b6d4"),
        ("Transfer", "transfer", "arrow-left-right", "#64748b"),
    ];
    for (name, typ, icon, color) in defaults {
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO categories (id,name,category_type,icon,color_hex) VALUES (?1,?2,?3,?4,?5)",
            params![id, name, typ, icon, color],
        )
        .map_err(|e| format!("seed: {e}"))?;
    }
    Ok(())
}

fn load_settings(conn: &Connection) -> (Option<String>, bool) {
    let url = conn
        .query_row(
            "SELECT value FROM settings WHERE key='sheet_url'",
            [],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    let auto: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key='auto_sync'",
            [],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    (url, auto.as_deref() == Some("1"))
}

pub fn save_settings(
    conn: &Connection,
    sheet_url: &Option<String>,
    auto_sync: bool,
) -> Result<(), String> {
    if let Some(u) = sheet_url {
        conn.execute(
            "INSERT OR REPLACE INTO settings(key,value) VALUES('sheet_url',?1)",
            params![u],
        )
        .map_err(|e| format!("save_url: {e}"))?;
    } else {
        conn.execute("DELETE FROM settings WHERE key='sheet_url'", [])
            .map_err(|e| format!("del_url: {e}"))?;
    }
    conn.execute(
        "INSERT OR REPLACE INTO settings(key,value) VALUES('auto_sync',?1)",
        params![if auto_sync { "1" } else { "0" }],
    )
    .map_err(|e| format!("save_auto: {e}"))?;
    Ok(())
}

fn row_to_account(row: &rusqlite::Row<'_>) -> rusqlite::Result<Account> {
    let t: String = row.get(2)?;
    let ia: i64 = row.get(5)?;
    Ok(Account {
        id: row.get(0)?,
        name: row.get(1)?,
        account_type: AccountType::from_str(&t),
        account_number: row.get(3)?,
        current_balance: row.get(4)?,
        is_active: ia != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn row_to_category(row: &rusqlite::Row<'_>) -> rusqlite::Result<Category> {
    let typ: String = row.get(2)?;
    Ok(Category {
        id: row.get(0)?,
        name: row.get(1)?,
        category_type: TxType::from_str(&typ),
        icon: row.get(3)?,
        color_hex: row.get(4)?,
    })
}

fn row_to_transaction(row: &rusqlite::Row<'_>) -> rusqlite::Result<PfTransaction> {
    let tt: String = row.get(5)?;
    let ss: String = row.get(9)?;
    Ok(PfTransaction {
        id: row.get(0)?,
        account_id: row.get(1)?,
        destination_account_id: row.get(2)?,
        category_id: row.get(3)?,
        amount: row.get(4)?,
        transaction_type: TxType::from_str(&tt),
        date: row.get(6)?,
        note: row.get(7)?,
        receipt_url: row.get(8)?,
        sync_status: SyncStatus::from_str(&ss),
        sheet_row_id: row.get(10)?,
    })
}

fn row_to_pocket(row: &rusqlite::Row<'_>) -> rusqlite::Result<SavingsPocket> {
    let lk: i64 = row.get(7)?;
    Ok(SavingsPocket {
        id: row.get(0)?,
        name: row.get(1)?,
        target_amount: row.get(2)?,
        current_amount: row.get(3)?,
        linked_account_id: row.get(4)?,
        target_date: row.get(5)?,
        color_tag: row.get(6)?,
        is_locked: lk != 0,
    })
}

pub fn accounts_all(conn: &Connection) -> Result<Vec<Account>, String> {
    let mut s = conn
        .prepare("SELECT id,name,account_type,account_number,current_balance,is_active,created_at,updated_at FROM accounts ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = s.query_map([], row_to_account).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn accounts_insert(conn: &Connection, a: &Account) -> Result<(), String> {
    conn.execute(
        "INSERT INTO accounts(id,name,account_type,account_number,current_balance,is_active,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![a.id, a.name, a.account_type.as_str(), a.account_number, a.current_balance, if a.is_active {1} else {0}, a.created_at, a.updated_at],
    ).map(|_| ()).map_err(|e| e.to_string())
}

pub fn accounts_update(conn: &Connection, a: &Account) -> Result<(), String> {
    conn.execute(
        "UPDATE accounts SET name=?1,account_type=?2,account_number=?3,current_balance=?4,is_active=?5,updated_at=?6 WHERE id=?7",
        params![a.name, a.account_type.as_str(), a.account_number, a.current_balance, if a.is_active {1} else {0}, a.updated_at, a.id],
    ).map(|_| ()).map_err(|e| e.to_string())
}

pub fn accounts_delete(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM accounts WHERE id=?1", params![id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn categories_all(conn: &Connection) -> Result<Vec<Category>, String> {
    let mut s = conn
        .prepare("SELECT id,name,category_type,icon,color_hex FROM categories ORDER BY name")
        .map_err(|e| e.to_string())?;
    let rows = s
        .query_map([], row_to_category)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn categories_insert(conn: &Connection, c: &Category) -> Result<(), String> {
    conn.execute(
        "INSERT INTO categories(id,name,category_type,icon,color_hex) VALUES(?1,?2,?3,?4,?5)",
        params![c.id, c.name, c.category_type.as_str(), c.icon, c.color_hex],
    )
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub fn categories_delete(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM categories WHERE id=?1", params![id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn transactions_all(
    conn: &Connection,
    limit: Option<i64>,
    offset: Option<i64>,
    search: Option<String>,
    account: Option<String>,
    category: Option<String>,
    tx_type: Option<String>,
    from: Option<i64>,
    to: Option<i64>,
) -> Result<Vec<PfTransaction>, String> {
    let mut sql = String::from("SELECT id,account_id,destination_account_id,category_id,amount,transaction_type,date,note,receipt_url,sync_status,sheet_row_id FROM transactions WHERE 1=1");
    let mut args: Vec<String> = vec![];
    if let Some(ref s) = search {
        if !s.is_empty() {
            sql.push_str(" AND note LIKE ?");
            args.push(format!("%{s}%"));
        }
    }
    if let Some(ref a) = account {
        if !a.is_empty() {
            sql.push_str(" AND account_id = ?");
            args.push(a.clone());
        }
    }
    if let Some(ref c) = category {
        if !c.is_empty() {
            sql.push_str(" AND category_id = ?");
            args.push(c.clone());
        }
    }
    if let Some(ref t) = tx_type {
        if !t.is_empty() {
            sql.push_str(" AND transaction_type = ?");
            args.push(t.clone());
        }
    }
    if let Some(f) = from {
        sql.push_str(" AND date >= ?");
        args.push(f.to_string());
    }
    if let Some(t) = to {
        sql.push_str(" AND date <= ?");
        args.push(t.to_string());
    }
    sql.push_str(" ORDER BY date DESC, rowid DESC");
    if let Some(l) = limit {
        sql.push_str(&format!(" LIMIT {l}"));
    }
    if let Some(o) = offset {
        sql.push_str(&format!(" OFFSET {o}"));
    }
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(args.iter()), row_to_transaction)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn transactions_insert(conn: &Connection, t: &PfTransaction) -> Result<(), String> {
    conn.execute(
        "INSERT INTO transactions(id,account_id,destination_account_id,category_id,amount,transaction_type,date,note,receipt_url,sync_status,sheet_row_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![t.id, t.account_id, t.destination_account_id, t.category_id, t.amount, t.transaction_type.as_str(), t.date, t.note, t.receipt_url, t.sync_status.as_str(), t.sheet_row_id],
    ).map(|_| ()).map_err(|e| e.to_string())
}

pub fn transactions_update(conn: &Connection, t: &PfTransaction) -> Result<(), String> {
    conn.execute(
        "UPDATE transactions SET account_id=?1,destination_account_id=?2,category_id=?3,amount=?4,transaction_type=?5,date=?6,note=?7,receipt_url=?8,sync_status=?9,sheet_row_id=?10 WHERE id=?11",
        params![t.account_id, t.destination_account_id, t.category_id, t.amount, t.transaction_type.as_str(), t.date, t.note, t.receipt_url, t.sync_status.as_str(), t.sheet_row_id, t.id],
    ).map(|_| ()).map_err(|e| e.to_string())
}

pub fn transactions_delete(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM transactions WHERE id=?1", params![id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn pockets_all(conn: &Connection) -> Result<Vec<SavingsPocket>, String> {
    let mut s = conn.prepare("SELECT id,name,target_amount,current_amount,linked_account_id,target_date,color_tag,is_locked FROM savings_pockets ORDER BY name").map_err(|e| e.to_string())?;
    let rows = s.query_map([], row_to_pocket).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn pockets_insert(conn: &Connection, p: &SavingsPocket) -> Result<(), String> {
    conn.execute(
        "INSERT INTO savings_pockets(id,name,target_amount,current_amount,linked_account_id,target_date,color_tag,is_locked) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
        params![p.id, p.name, p.target_amount, p.current_amount, p.linked_account_id, p.target_date, p.color_tag, if p.is_locked {1} else {0}],
    ).map(|_| ()).map_err(|e| e.to_string())
}

pub fn pockets_update(conn: &Connection, p: &SavingsPocket) -> Result<(), String> {
    conn.execute(
        "UPDATE savings_pockets SET name=?1,target_amount=?2,current_amount=?3,linked_account_id=?4,target_date=?5,color_tag=?6,is_locked=?7 WHERE id=?8",
        params![p.name, p.target_amount, p.current_amount, p.linked_account_id, p.target_date, p.color_tag, if p.is_locked {1} else {0}, p.id],
    ).map(|_| ()).map_err(|e| e.to_string())
}

pub fn pockets_delete(conn: &Connection, id: &str) -> Result<(), String> {
    // Return the pocket's funds to the linked account before deleting.
    let before: Option<(i64, String)> = conn
        .query_row(
            "SELECT current_amount, linked_account_id FROM savings_pockets WHERE id=?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if let Some((amt, acct)) = before {
        if amt > 0 {
            conn.execute("UPDATE accounts SET current_balance = current_balance + ?1, updated_at=?2 WHERE id=?3",
                params![amt, chrono::Utc::now().timestamp(), acct]).map_err(|e| e.to_string())?;
        }
    }
    conn.execute("DELETE FROM savings_pockets WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn pockets_one(conn: &Connection, id: &str) -> Result<Option<SavingsPocket>, String> {
    conn.query_row("SELECT id,name,target_amount,current_amount,linked_account_id,target_date,color_tag,is_locked FROM savings_pockets WHERE id=?1", params![id], row_to_pocket)
        .optional().map_err(|e| e.to_string())
}

pub fn dashboard(
    conn: &Connection,
    month_start: i64,
    month_end: i64,
) -> Result<DashboardSummary, String> {
    let net_worth: i64 = conn.query_row(
        "SELECT COALESCE((SELECT SUM(current_balance) FROM accounts WHERE is_active=1),0) + COALESCE((SELECT SUM(current_amount) FROM savings_pockets),0)",
        [], |r| r.get(0)).unwrap_or(0);
    let savings: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(current_amount),0) FROM savings_pockets",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let inc: i64 = conn.query_row("SELECT COALESCE(SUM(amount),0) FROM transactions WHERE transaction_type='income' AND date>=?1 AND date<=?2", params![month_start, month_end], |r| r.get(0)).unwrap_or(0);
    let exp: i64 = conn.query_row("SELECT COALESCE(SUM(amount),0) FROM transactions WHERE transaction_type='expense' AND date>=?1 AND date<=?2", params![month_start, month_end], |r| r.get(0)).unwrap_or(0);
    Ok(DashboardSummary {
        net_worth,
        total_income: inc,
        total_expense: exp,
        net_cashflow: inc - exp,
        total_savings: savings,
    })
}

pub fn expense_by_category(
    conn: &Connection,
    from: i64,
    to: i64,
) -> Result<Vec<CategorySpend>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT t.category_id, c.name, c.color_hex, c.icon, SUM(t.amount) as tot
         FROM transactions t JOIN categories c ON c.id=t.category_id
         WHERE t.transaction_type='expense' AND t.date>=?1 AND t.date<=?2
         GROUP BY t.category_id ORDER BY tot DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![from, to], |r| {
            Ok(CategorySpend {
                category_id: r.get(0)?,
                category_name: r.get(1)?,
                color_hex: r.get(2)?,
                icon: r.get(3)?,
                amount: r.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn export_transactions_csv(
    conn: &Connection,
    from: Option<i64>,
    to: Option<i64>,
) -> Result<String, String> {
    let txs = transactions_all(conn, None, None, None, None, None, None, from, to)?;
    let cats = categories_all(conn).unwrap_or_default();
    let accts = accounts_all(conn).unwrap_or_default();
    let mut out = String::from("id,tanggal,tipe,akun,kategori,nominal,catatan\n");
    for t in &txs {
        let acct = accts
            .iter()
            .find(|a| a.id == t.account_id)
            .map(|a| a.name.as_str())
            .unwrap_or("-");
        let cat = cats
            .iter()
            .find(|c| c.id == t.category_id)
            .map(|c| c.name.as_str())
            .unwrap_or("-");
        let typ = t.transaction_type.as_str();
        let date_str = chrono::DateTime::from_timestamp(t.date, 0)
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_default();
        let note = t.note.replace('"', "''").replace('\n', " ");
        out.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            t.id,
            date_str,
            typ,
            acct,
            cat,
            t.amount,
            format!("\"{note}\"")
        ));
    }
    Ok(out)
}

pub fn pending_count(conn: &Connection) -> u32 {
    conn.query_row(
        "SELECT COUNT(*) FROM transactions WHERE sync_status='pending' OR sync_status='failed'",
        [],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

pub fn reset_all_data(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "DELETE FROM transactions; DELETE FROM savings_pockets; DELETE FROM accounts;",
    )
    .map_err(|e| e.to_string())
}
