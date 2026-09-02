use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

use crate::models::{
    Account, AccountType, Category, CategorySpend, DashboardSummary, InvestmentTransaction,
    MutualFundProduct, Obligation, ObligationSummary, PortfolioSnapshot, SavingsPocket, SyncStatus,
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
    // Deteksi sebelum CREATE: apakah FTS5 sudah ada di DB ini (DB lama = belum).
    // Catatan: COUNT(*) pada external-content FTS5 membaca tabel content, BUKAN
    // jumlah baris index — jadi COUNT tidak bisa dipakai untuk deteksi index kosong.
    let fts_existed: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='bibit_products_fts')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(false);
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
        CREATE TABLE IF NOT EXISTS bibit_products_cache (
          id TEXT PRIMARY KEY, name TEXT NOT NULL, fund_type TEXT NOT NULL,
          manager_name TEXT NOT NULL, is_syariah INTEGER NOT NULL DEFAULT 0,
          current_nav REAL NOT NULL, return_1d REAL, return_1y REAL, aum REAL,
          min_buy INTEGER DEFAULT 10000, last_fetched_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS investment_transactions (
          id TEXT PRIMARY KEY, product_id TEXT NOT NULL, account_id TEXT NOT NULL,
          tx_type TEXT NOT NULL, units REAL NOT NULL, nav_per_unit REAL NOT NULL,
          total_amount INTEGER NOT NULL, fee INTEGER NOT NULL DEFAULT 0,
          date INTEGER NOT NULL, note TEXT,
          FOREIGN KEY(product_id) REFERENCES bibit_products_cache(id),
          FOREIGN KEY(account_id) REFERENCES accounts(id)
        );
        CREATE TABLE IF NOT EXISTS portfolio_daily_snapshots (
          day INTEGER PRIMARY KEY,
          total_value INTEGER NOT NULL,
          total_invested INTEGER NOT NULL,
          unrealized_pnl INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS obligations (
          id TEXT PRIMARY KEY, direction TEXT NOT NULL, counterparty TEXT NOT NULL, title TEXT NOT NULL,
          original_amount INTEGER NOT NULL, remaining_amount INTEGER NOT NULL, due_date INTEGER,
          note TEXT, status TEXT NOT NULL DEFAULT 'OPEN', created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_bibit_name ON bibit_products_cache(name);
        CREATE INDEX IF NOT EXISTS idx_bibit_type ON bibit_products_cache(fund_type);
        CREATE VIRTUAL TABLE IF NOT EXISTS bibit_products_fts USING fts5(
          name, manager_name,
          content='bibit_products_cache', content_rowid='rowid',
          tokenize='trigram'
        );
        CREATE TRIGGER IF NOT EXISTS bibit_ai AFTER INSERT ON bibit_products_cache BEGIN
          INSERT INTO bibit_products_fts(rowid, name, manager_name)
          VALUES (new.rowid, new.name, new.manager_name);
        END;
        CREATE TRIGGER IF NOT EXISTS bibit_ad AFTER DELETE ON bibit_products_cache BEGIN
          INSERT INTO bibit_products_fts(bibit_products_fts, rowid, name, manager_name)
          VALUES('delete', old.rowid, old.name, old.manager_name);
        END;
        CREATE TRIGGER IF NOT EXISTS bibit_au AFTER UPDATE ON bibit_products_cache BEGIN
          INSERT INTO bibit_products_fts(bibit_products_fts, rowid, name, manager_name)
          VALUES('delete', old.rowid, old.name, old.manager_name);
          INSERT INTO bibit_products_fts(rowid, name, manager_name)
          VALUES (new.rowid, new.name, new.manager_name);
        END;
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

    // Backfill FTS untuk DB lama yang sudah punya cache sebelum FTS5 diperkenalkan:
    // virtual table baru dibuat kosong dan trigger hanya menjaga tulis berikutnya,
    // jadi tanpa rebuild ini search >=3 char return 0 sampai sync berikutnya.
    // Ter-guard: hanya saat FTS baru saja dibuat (fresh install / cache kosong → no-op).
    if !fts_existed {
        conn.execute_batch("INSERT INTO bibit_products_fts(bibit_products_fts) VALUES('rebuild');")
            .map_err(|e| format!("fts backfill: {e}"))?;
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

// ── Bibit / Reksa Dana helpers ───────────────────────────────────────────────

/// Maps one `bibit_products_cache` row to a `MutualFundProduct`.
///
/// `return_1d`, `return_1y`, `aum` are read as `Option<f64>` because the cache
/// columns are nullable (Bibit omits metrics for young funds).
fn row_to_bibit_product(r: &rusqlite::Row<'_>) -> Result<MutualFundProduct, rusqlite::Error> {
    Ok(MutualFundProduct {
        id: r.get("id")?,
        name: r.get("name")?,
        fund_type: r.get("fund_type")?,
        manager_name: r.get("manager_name")?,
        is_syariah: r.get::<_, i64>("is_syariah")? != 0,
        current_nav: r.get("current_nav")?,
        return_1d: r.get::<_, Option<f64>>("return_1d")?,
        return_1y: r.get::<_, Option<f64>>("return_1y")?,
        aum: r.get::<_, Option<f64>>("aum")?,
        min_buy: r.get("min_buy")?,
        last_fetched_at: r.get("last_fetched_at")?,
    })
}

/// Maps one `investment_transactions` row to an `InvestmentTransaction`.
/// `note` stays optional — the UI leaves it blank instead of storing an empty string.
fn row_to_investment_tx(r: &rusqlite::Row<'_>) -> Result<InvestmentTransaction, rusqlite::Error> {
    Ok(InvestmentTransaction {
        id: r.get("id")?,
        product_id: r.get("product_id")?,
        account_id: r.get("account_id")?,
        tx_type: r.get("tx_type")?,
        units: r.get("units")?,
        nav_per_unit: r.get("nav_per_unit")?,
        total_amount: r.get("total_amount")?,
        fee: r.get("fee")?,
        date: r.get("date")?,
        note: r.get::<_, Option<String>>("note")?,
    })
}

/// Inserts a product, or refreshes every field of the existing row on `id` conflict.
///
/// Keeps the cache idempotent so repeated polling of the Bibit catalogue never
/// duplicates rows; `last_fetched_at` in the payload marks the snapshot age.
pub fn bibit_product_upsert(conn: &Connection, p: &MutualFundProduct) -> Result<(), String> {
    conn.execute(
        "INSERT INTO bibit_products_cache (
            id, name, fund_type, manager_name, is_syariah, current_nav,
            return_1d, return_1y, aum, min_buy, last_fetched_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
         ON CONFLICT(id) DO UPDATE SET
            name=excluded.name, fund_type=excluded.fund_type,
            manager_name=excluded.manager_name, is_syariah=excluded.is_syariah,
            current_nav=excluded.current_nav, return_1d=excluded.return_1d,
            return_1y=excluded.return_1y, aum=excluded.aum,
            min_buy=excluded.min_buy, last_fetched_at=excluded.last_fetched_at",
        params![
            p.id,
            p.name,
            p.fund_type,
            p.manager_name,
            p.is_syariah as i64,
            p.current_nav,
            p.return_1d,
            p.return_1y,
            p.aum,
            p.min_buy,
            p.last_fetched_at
        ],
    )
    .map_err(|e| format!("bibit_product_upsert: {e}"))?;
    Ok(())
}

/// Rebuilds the FTS index from the cache table in one pass.
///
/// Per-row triggers keep the index fresh during individual upserts, but rebuilding
/// once is far cheaper than 2943 incremental updates after a bulk catalogue sync.
#[allow(dead_code)]
pub fn rebuild_bibit_fts(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "INSERT INTO bibit_products_fts(bibit_products_fts) VALUES('rebuild')",
        [],
    )
    .map(|_| ())
    .map_err(|e| format!("rebuild_bibit_fts: {e}"))
}

/// Searches the cached catalogue by product/manager name, optionally narrowed to one fund type.
///
/// Empty query returns the 5 alphabetically-first products; ≥3 chars uses the FTS5 trigram
/// index (quoted so FTS operators stay literal); 1–2 chars falls back to LIKE because the
/// trigram index cannot index such short terms. Capped at 50 hits, sorted by name.
pub fn bibit_product_search(
    conn: &Connection,
    query: &str,
    fund_type: Option<&str>,
) -> Result<Vec<MutualFundProduct>, String> {
    let trimmed = query.trim();
    let (sql, p1): (&str, String) = if trimmed.is_empty() {
        (
            "SELECT * FROM bibit_products_cache
             WHERE (?2 IS NULL OR fund_type = ?2)
             ORDER BY name LIMIT 5",
            String::new(),
        )
    } else if trimmed.chars().count() >= 3 {
        // Wrap in double quotes ("" escapes an embedded quote) so user text can never be
        // parsed as FTS5 operators and multi-word input stays one ordered phrase.
        (
            "SELECT c.* FROM bibit_products_cache c
             JOIN bibit_products_fts f ON f.rowid = c.rowid
             WHERE bibit_products_fts MATCH ?1
               AND (?2 IS NULL OR c.fund_type = ?2)
             ORDER BY c.name LIMIT 50",
            format!("\"{}\"", trimmed.replace('"', "\"\"")),
        )
    } else {
        // Escape backslash duluan (jika tidak, \% akan berubah lagi jadi \\%),
        // lalu % dan _ supaya wildcard user diperlakukan literal, sama seperti
        // cabang FTS. ESCAPE '\' di SQL mendeklarasikan karakter escape-nya.
        let escaped = trimmed
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        (
            "SELECT * FROM bibit_products_cache
             WHERE (name LIKE '%'||?1||'%' ESCAPE '\\' OR manager_name LIKE '%'||?1||'%' ESCAPE '\\')
               AND (?2 IS NULL OR fund_type = ?2)
             ORDER BY name LIMIT 50",
            escaped,
        )
    };
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("bibit_product_search: {e}"))?;
    let rows = stmt
        .query_map(params![p1, fund_type], row_to_bibit_product)
        .map_err(|e| format!("bibit_product_search: {e}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("bibit_product_search: {e}"))?);
    }
    Ok(out)
}

/// Returns the whole cached catalogue sorted by name — used for the product list view.
pub fn bibit_products_all(conn: &Connection) -> Result<Vec<MutualFundProduct>, String> {
    let mut stmt = conn
        .prepare("SELECT * FROM bibit_products_cache ORDER BY name")
        .map_err(|e| format!("bibit_products_all: {e}"))?;
    let rows = stmt
        .query_map([], row_to_bibit_product)
        .map_err(|e| format!("bibit_products_all: {e}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("bibit_products_all: {e}"))?);
    }
    Ok(out)
}

/// Records one BUY / SELL / DIVIDEND transaction for the portfolio aggregation.
pub fn investment_tx_insert(conn: &Connection, t: &InvestmentTransaction) -> Result<(), String> {
    conn.execute(
        "INSERT INTO investment_transactions (
            id, product_id, account_id, tx_type, units, nav_per_unit,
            total_amount, fee, date, note
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            t.id,
            t.product_id,
            t.account_id,
            t.tx_type,
            t.units,
            t.nav_per_unit,
            t.total_amount,
            t.fee,
            t.date,
            t.note
        ],
    )
    .map_err(|e| format!("investment_tx_insert: {e}"))?;
    Ok(())
}

/// Lists every investment transaction, newest first — feeds the riwayat investment list.
pub fn investment_tx_all(conn: &Connection) -> Result<Vec<InvestmentTransaction>, String> {
    let mut stmt = conn
        .prepare("SELECT * FROM investment_transactions ORDER BY date DESC")
        .map_err(|e| format!("investment_tx_all: {e}"))?;
    let rows = stmt
        .query_map([], row_to_investment_tx)
        .map_err(|e| format!("investment_tx_all: {e}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("investment_tx_all: {e}"))?);
    }
    Ok(out)
}

/// Lists transactions of one product, newest first — per-holding transaction drill-down.
#[allow(dead_code)] // disiapkan untuk UI drill-down holding (belum dipakai command)
pub fn investment_tx_by_product(
    conn: &Connection,
    product_id: &str,
) -> Result<Vec<InvestmentTransaction>, String> {
    let mut stmt = conn
        .prepare("SELECT * FROM investment_transactions WHERE product_id=?1 ORDER BY date DESC")
        .map_err(|e| format!("investment_tx_by_product: {e}"))?;
    let rows = stmt
        .query_map(params![product_id], row_to_investment_tx)
        .map_err(|e| format!("investment_tx_by_product: {e}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("investment_tx_by_product: {e}"))?);
    }
    Ok(out)
}

fn row_to_obligation(row: &rusqlite::Row<'_>) -> rusqlite::Result<Obligation> {
    Ok(Obligation {
        id: row.get(0)?,
        direction: row.get(1)?,
        counterparty: row.get(2)?,
        title: row.get(3)?,
        original_amount: row.get(4)?,
        remaining_amount: row.get(5)?,
        due_date: row.get(6)?,
        note: row.get(7)?,
        status: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub fn obligations_all(conn: &Connection) -> Result<Vec<Obligation>, String> {
    let mut stmt = conn
        .prepare("SELECT id,direction,counterparty,title,original_amount,remaining_amount,due_date,note,status,created_at,updated_at FROM obligations ORDER BY status ASC, due_date ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], row_to_obligation)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn obligation_one(conn: &Connection, id: &str) -> Result<Option<Obligation>, String> {
    conn.query_row(
        "SELECT id,direction,counterparty,title,original_amount,remaining_amount,due_date,note,status,created_at,updated_at FROM obligations WHERE id=?1",
        params![id], row_to_obligation,
    ).optional().map_err(|e| e.to_string())
}

pub fn obligation_insert(conn: &Connection, o: &Obligation) -> Result<(), String> {
    conn.execute("INSERT INTO obligations(id,direction,counterparty,title,original_amount,remaining_amount,due_date,note,status,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)", params![o.id, o.direction, o.counterparty, o.title, o.original_amount, o.remaining_amount, o.due_date, o.note, o.status, o.created_at, o.updated_at]).map(|_| ()).map_err(|e| e.to_string())
}

pub fn obligation_update(conn: &Connection, o: &Obligation) -> Result<(), String> {
    conn.execute("UPDATE obligations SET direction=?1,counterparty=?2,title=?3,original_amount=?4,remaining_amount=?5,due_date=?6,note=?7,status=?8,updated_at=?9 WHERE id=?10", params![o.direction, o.counterparty, o.title, o.original_amount, o.remaining_amount, o.due_date, o.note, o.status, o.updated_at, o.id]).map(|_| ()).map_err(|e| e.to_string())
}

pub fn obligation_delete(conn: &Connection, id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM obligations WHERE id=?1", params![id])
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn obligation_summary(conn: &Connection) -> Result<ObligationSummary, String> {
    conn.query_row("SELECT COALESCE(SUM(CASE WHEN status='OPEN' AND direction='DEBT' THEN remaining_amount ELSE 0 END),0), COALESCE(SUM(CASE WHEN status='OPEN' AND direction='RECEIVABLE' THEN remaining_amount ELSE 0 END),0), COALESCE(SUM(CASE WHEN status='OPEN' AND due_date IS NOT NULL AND due_date < ?1 THEN 1 ELSE 0 END),0) FROM obligations", params![chrono::Utc::now().timestamp()], |r| Ok(ObligationSummary { total_debt: r.get(0)?, total_receivable: r.get(1)?, overdue_count: r.get(2)? })).map_err(|e| e.to_string())
}

// ── Snapshot portofolio harian ───────────────────────────────────────────────

/// Insert or replace the daily portfolio snapshot (one row per local midnight).
pub fn snapshot_upsert(
    conn: &Connection,
    day: i64,
    total_value: i64,
    total_invested: i64,
    unrealized_pnl: i64,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO portfolio_daily_snapshots
         (day, total_value, total_invested, unrealized_pnl)
         VALUES (?1, ?2, ?3, ?4)",
        params![day, total_value, total_invested, unrealized_pnl],
    )
    .map_err(|e| format!("snapshot_upsert: {e}"))?;
    Ok(())
}

/// Returns the most recent daily snapshots, newest day first.
pub fn snapshots_last(conn: &Connection, days: i64) -> Result<Vec<PortfolioSnapshot>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT day, total_value, total_invested, unrealized_pnl
             FROM portfolio_daily_snapshots ORDER BY day DESC LIMIT ?1",
        )
        .map_err(|e| format!("snapshots_last: {e}"))?;
    let rows = stmt
        .query_map(params![days], |r| {
            Ok(PortfolioSnapshot {
                day: r.get(0)?,
                total_value: r.get(1)?,
                total_invested: r.get(2)?,
                unrealized_pnl: r.get(3)?,
            })
        })
        .map_err(|e| format!("snapshots_last: {e}"))?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row.map_err(|e| format!("snapshots_last: {e}"))?);
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
    // investment_transactions dihapus lebih dulu karena ber-FK ke accounts dan
    // bibit_products_cache (PRAGMA foreign_keys=ON akan menolak urutan terbalik).
    conn.execute_batch(
        "DELETE FROM investment_transactions; DELETE FROM bibit_products_cache; DELETE FROM transactions; DELETE FROM savings_pockets; DELETE FROM accounts; DELETE FROM obligations;",
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod bibit_search_tests {
    use super::*;

    /// Opens an in-memory DB through `init_db` and seeds three catalogue products.
    fn seeded() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        for (i, name) in ["Mandiri Investa BCA", "BCA Prima Dana", "Reksa Dana Sukun"]
            .iter()
            .enumerate()
        {
            let p = MutualFundProduct {
                id: format!("p{i}"),
                name: name.to_string(),
                fund_type: "Pasar Uang".into(),
                manager_name: "Bca".into(),
                is_syariah: false,
                current_nav: 1.0,
                return_1d: None,
                return_1y: None,
                aum: None,
                min_buy: 10000,
                last_fetched_at: 0,
            };
            bibit_product_upsert(&conn, &p).unwrap();
        }
        conn
    }

    fn names(v: &Vec<MutualFundProduct>) -> Vec<String> {
        v.iter().map(|x| x.name.clone()).collect()
    }

    /// Trigram path (≥3 chars): mid-string and case-insensitive match, alphabetical order,
    /// FTS operator text treated literally, fund_type filter, and trigger-driven refresh.
    #[test]
    fn fts_matches_substrings_and_survives_operator_text() {
        let c = seeded();
        assert_eq!(
            names(&bibit_product_search(&c, "bca", None).unwrap()),
            ["BCA Prima Dana", "Mandiri Investa BCA", "Reksa Dana Sukun"],
            "trigram must match mid-name and inside manager_name, case-insensitive, sorted by name"
        );
        assert_eq!(
            names(&bibit_product_search(&c, "MANDIRI", None).unwrap()),
            ["Mandiri Investa BCA"]
        );
        assert_eq!(
            names(&bibit_product_search(&c, "investa bca", None).unwrap()),
            ["Mandiri Investa BCA"],
            "multi-word query must stay one ordered phrase"
        );
        assert_eq!(
            bibit_product_search(&c, "' OR 1=1 --\"", None)
                .unwrap()
                .len(),
            0,
            "quoted MATCH must not error nor match everything"
        );
        assert_eq!(
            bibit_product_search(&c, "bca", Some("Obligasi"))
                .unwrap()
                .len(),
            0
        );
        let mut p = bibit_product_search(&c, "sukun", None).unwrap().remove(0);
        p.name = "Renamed Foo".into();
        bibit_product_upsert(&c, &p).unwrap();
        assert_eq!(bibit_product_search(&c, "sukun", None).unwrap().len(), 0);
        assert_eq!(bibit_product_search(&c, "renamed", None).unwrap().len(), 1);
    }

    /// LIKE fallback (1–2 chars): trigram cannot index such short terms, so they must still
    /// return substring hits instead of an empty picker.
    #[test]
    fn short_query_falls_back_to_like() {
        let c = seeded();
        assert_eq!(bibit_product_search(&c, "bc", None).unwrap().len(), 3);
        assert_eq!(bibit_product_search(&c, "su", None).unwrap().len(), 1);
        assert!(bibit_product_search(&c, "%", None).unwrap().is_empty());
        assert!(bibit_product_search(&c, "_", None).unwrap().is_empty());
    }

    /// Empty query keeps the 5-first alphabetical default list.
    #[test]
    fn empty_query_returns_defaults() {
        let c = seeded();
        assert_eq!(bibit_product_search(&c, "   ", None).unwrap().len(), 3);
    }

    /// Bulk rebuild repopulates the index from the content table.
    #[test]
    fn rebuild_resyncs_index() {
        let c = seeded();
        rebuild_bibit_fts(&c).unwrap();
        assert_eq!(bibit_product_search(&c, "bca", None).unwrap().len(), 3);
    }
}

#[cfg(test)]
mod obligations_tests {
    use super::*;

    fn open_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_db(&conn).unwrap();
        conn
    }

    fn obligation(id: &str, direction: &str, amount: i64, status: &str) -> Obligation {
        Obligation {
            id: id.into(),
            direction: direction.into(),
            counterparty: "Bank".into(),
            title: "Pinjaman".into(),
            original_amount: amount,
            remaining_amount: amount,
            due_date: None,
            note: None,
            status: status.into(),
            created_at: 0,
            updated_at: 0,
        }
    }

    #[test]
    fn crud_and_summary_work() {
        let c = open_db();
        let o1 = obligation("o1", "DEBT", 100, "OPEN");
        let o2 = obligation("o2", "RECEIVABLE", 200, "OPEN");
        obligation_insert(&c, &o1).unwrap();
        obligation_insert(&c, &o2).unwrap();
        assert_eq!(obligations_all(&c).unwrap().len(), 2);
        let s = obligation_summary(&c).unwrap();
        assert_eq!(s.total_debt, 100);
        assert_eq!(s.total_receivable, 200);
        assert_eq!(s.overdue_count, 0);

        let mut o1 = o1;
        o1.remaining_amount = 50;
        obligation_update(&c, &o1).unwrap();
        assert_eq!(
            obligation_one(&c, "o1").unwrap().unwrap().remaining_amount,
            50
        );

        obligation_delete(&c, "o2").unwrap();
        assert!(obligation_one(&c, "o2").unwrap().is_none());
        let s = obligation_summary(&c).unwrap();
        assert_eq!(s.total_debt, 50);
        assert_eq!(s.total_receivable, 0);
    }

    #[test]
    fn summary_counts_overdue_open() {
        let c = open_db();
        let mut o = obligation("o1", "DEBT", 100, "OPEN");
        o.due_date = Some(chrono::Utc::now().timestamp() - 1);
        obligation_insert(&c, &o).unwrap();
        let s = obligation_summary(&c).unwrap();
        assert_eq!(s.overdue_count, 1);
        o.status = "DONE".into();
        obligation_update(&c, &o).unwrap();
        let s = obligation_summary(&c).unwrap();
        assert_eq!(s.overdue_count, 0);
    }

    #[test]
    fn reset_all_data_clears_obligations() {
        let c = open_db();
        obligation_insert(&c, &obligation("o1", "DEBT", 100, "OPEN")).unwrap();
        reset_all_data(&c).unwrap();
        assert_eq!(obligations_all(&c).unwrap().len(), 0);
    }
}
