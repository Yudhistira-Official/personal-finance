use serde::{Deserialize, Serialize};

/// Tipe akun / dompet keuangan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    Bank,
    EWallet,
    Cash,
    Investment,
}

impl AccountType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountType::Bank => "bank",
            AccountType::EWallet => "ewallet",
            AccountType::Cash => "cash",
            AccountType::Investment => "investment",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "bank" => AccountType::Bank,
            "ewallet" => AccountType::EWallet,
            "cash" => AccountType::Cash,
            "investment" => AccountType::Investment,
            _ => AccountType::Bank,
        }
    }

    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            AccountType::Bank => "Bank",
            AccountType::EWallet => "E-Wallet",
            AccountType::Cash => "Cash",
            AccountType::Investment => "Investasi",
        }
    }
}

/// Tipe transaksi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TxType {
    Income,
    Expense,
    Transfer,
}

impl TxType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TxType::Income => "income",
            TxType::Expense => "expense",
            TxType::Transfer => "transfer",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "income" => TxType::Income,
            "expense" => TxType::Expense,
            "transfer" => TxType::Transfer,
            _ => TxType::Expense,
        }
    }

    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            TxType::Income => "Pemasukan",
            TxType::Expense => "Pengeluaran",
            TxType::Transfer => "Transfer",
        }
    }
}

/// Status sinkronisasi transaksi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Synced,
    Pending,
    Failed,
}

impl SyncStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SyncStatus::Synced => "synced",
            SyncStatus::Pending => "pending",
            SyncStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "synced" => SyncStatus::Synced,
            "failed" => SyncStatus::Failed,
            _ => SyncStatus::Pending,
        }
    }
}

/// Akun / Dompet Keuangan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub account_type: AccountType,
    pub account_number: Option<String>,
    pub current_balance: i64,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Kategori Transaksi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub category_type: TxType,
    pub icon: String,
    pub color_hex: String,
}

/// Transaksi.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub account_id: String,
    pub destination_account_id: Option<String>,
    pub category_id: String,
    pub amount: i64,
    pub transaction_type: TxType,
    pub date: i64,
    pub note: String,
    pub receipt_url: Option<String>,
    pub sync_status: SyncStatus,
    pub sheet_row_id: Option<u32>,
}

/// Pos Tabungan (Savings Pocket).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavingsPocket {
    pub id: String,
    pub name: String,
    pub target_amount: i64,
    pub current_amount: i64,
    pub linked_account_id: String,
    pub target_date: Option<i64>,
    pub color_tag: String,
    pub is_locked: bool,
}

/// Input untuk membuat / mengupdate akun (dari frontend).
#[derive(Debug, Clone, Deserialize)]
pub struct AccountInput {
    pub name: String,
    pub account_type: String,
    pub account_number: Option<String>,
    pub current_balance: i64,
    pub is_active: Option<bool>,
}

/// Input untuk membuat / mengupdate kategori.
#[derive(Debug, Clone, Deserialize)]
pub struct CategoryInput {
    pub name: String,
    pub category_type: String,
    pub icon: String,
    pub color_hex: String,
}

/// Input untuk membuat / mengupdate transaksi.
#[derive(Debug, Clone, Deserialize)]
pub struct TransactionInput {
    pub account_id: String,
    pub destination_account_id: Option<String>,
    pub category_id: String,
    pub amount: i64,
    pub transaction_type: String,
    pub date: i64,
    pub note: String,
    pub receipt_url: Option<String>,
}

/// Input untuk membuat / mengupdate pos tabungan.
#[derive(Debug, Clone, Deserialize)]
pub struct SavingsPocketInput {
    pub name: String,
    pub target_amount: i64,
    pub current_amount: Option<i64>,
    pub linked_account_id: String,
    pub target_date: Option<i64>,
    pub color_tag: String,
    pub is_locked: Option<bool>,
}

/// Ringkasan dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct DashboardSummary {
    pub net_worth: i64,
    pub total_income: i64,
    pub total_expense: i64,
    pub net_cashflow: i64,
    pub total_savings: i64,
}

/// Ringkasan pengeluaran per kategori (untuk donut chart).
#[derive(Debug, Clone, Serialize)]
pub struct CategorySpend {
    pub category_id: String,
    pub category_name: String,
    pub color_hex: String,
    pub icon: String,
    pub amount: i64,
}

/// Status sinkronisasi keseluruhan.
#[derive(Debug, Clone, Serialize)]
pub struct SyncInfo {
    pub status: String,
    pub pending_count: u32,
    pub sheet_url: Option<String>,
}
