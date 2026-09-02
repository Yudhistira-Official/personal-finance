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
    pub auto_sync: bool,
}

/// Produk reksa dana (cache dari Bibit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutualFundProduct {
    pub id: String,
    pub name: String,
    pub fund_type: String,
    pub manager_name: String,
    pub is_syariah: bool,
    pub current_nav: f64,
    pub return_1d: Option<f64>,
    pub return_1y: Option<f64>,
    pub aum: Option<f64>,
    pub min_buy: i64,
    pub last_fetched_at: i64,
}

/// Transaksi investasi reksa dana.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestmentTransaction {
    pub id: String,
    pub product_id: String,
    pub account_id: String,
    pub tx_type: String,
    pub units: f64,
    pub nav_per_unit: f64,
    pub total_amount: i64,
    pub fee: i64,
    pub date: i64,
    pub note: Option<String>,
}

/// Ringkasan kepemilikan portofolio (hasil agregasi DCA).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioHolding {
    pub product_id: String,
    pub product_name: String,
    pub fund_type: String,
    pub manager_name: String,
    pub total_units: f64,
    pub avg_buy_nav: f64,
    pub total_invested: i64,
    pub current_nav: f64,
    pub current_value: i64,
    pub unrealized_pnl: i64,
    pub roi_percentage: f64,
}

/// Snapshot nilai portofolio harian (akumulasi dana dari modal awal).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    /// Unix timestamp tengah malam lokal (awal hari) snapshot direkam.
    pub day: i64,
    pub total_value: i64,
    pub total_invested: i64,
    pub unrealized_pnl: i64,
}

/// Hutang / Piutang terhadap satu pihak (orang, toko, institusi).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obligation {
    pub id: String,
    pub direction: String, // "DEBT" (hutang ke orang lain) | "RECEIVABLE" (piutang)
    pub counterparty: String,
    pub title: String,
    pub original_amount: i64,
    pub remaining_amount: i64,
    pub due_date: Option<i64>,
    pub note: Option<String>,
    pub status: String, // "OPEN" | "DONE"
    pub created_at: i64,
    pub updated_at: i64,
}

/// Input untuk membuat / mengupdate hutang-piutang.
#[derive(Debug, Clone, Deserialize)]
pub struct ObligationInput {
    pub direction: String,
    pub counterparty: String,
    pub title: String,
    pub original_amount: i64,
    pub remaining_amount: Option<i64>, // default = original_amount
    pub due_date: Option<i64>,
    pub note: Option<String>,
}

/// Pembayaran (sebagian / lunas) atas satu hutang-piutang.
#[derive(Debug, Clone, Deserialize)]
pub struct ObligationPayment {
    pub obligation_id: String,
    pub amount: i64,                // > 0
    pub account_id: Option<String>, // bila Some: catat transaksi kas + ubah saldo
    pub date: Option<i64>,
}

/// Ringkasan hutang-piutang yang masih OPEN.
#[derive(Debug, Clone, Serialize)]
pub struct ObligationSummary {
    pub total_debt: i64,       // sum remaining OPEN DEBT
    pub total_receivable: i64, // sum remaining OPEN RECEIVABLE
    pub overdue_count: u32,    // OPEN dengan due_date < now
}

/// Ringkasan nilai portofolio investasi.
#[derive(Debug, Clone, Serialize)]
pub struct PortfolioSummary {
    pub total_value: i64,
    pub total_invested: i64,
    pub unrealized_pnl: i64,
}
