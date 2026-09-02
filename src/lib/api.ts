import { invoke } from "@tauri-apps/api/core";

export type AccountType = "bank" | "ewallet" | "cash" | "investment";
export type TxType = "income" | "expense" | "transfer";

export interface Account {
  id: string;
  name: string;
  account_type: AccountType;
  account_number: string | null;
  current_balance: number;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export interface Category {
  id: string;
  name: string;
  category_type: TxType;
  icon: string;
  color_hex: string;
}

export interface Transaction {
  id: string;
  account_id: string;
  destination_account_id: string | null;
  category_id: string;
  amount: number;
  transaction_type: TxType;
  date: number;
  note: string;
  receipt_url: string | null;
  sync_status: "synced" | "pending" | "failed";
  sheet_row_id: number | null;
}

export interface SavingsPocket {
  id: string;
  name: string;
  target_amount: number;
  current_amount: number;
  linked_account_id: string;
  target_date: number | null;
  color_tag: string;
  is_locked: boolean;
}

export interface DashboardSummary {
  net_worth: number;
  total_income: number;
  total_expense: number;
  net_cashflow: number;
  total_savings: number;
}

export interface CategorySpend {
  category_id: string;
  category_name: string;
  color_hex: string;
  icon: string;
  amount: number;
}

export interface SyncInfo {
  status: string;
  pending_count: number;
  sheet_url: string | null;
  auto_sync: boolean;
}

export interface MutualFundProduct {
  id: string;
  name: string;
  fund_type: string;
  manager_name: string;
  is_syariah: boolean;
  current_nav: number;
  return_1d: number | null;
  return_1y: number | null;
  aum: number | null;
  min_buy: number;
  last_fetched_at: number;
}

export interface InvestmentTransaction {
  id: string;
  product_id: string;
  account_id: string;
  tx_type: string;
  units: number;
  nav_per_unit: number;
  total_amount: number;
  fee: number;
  date: number;
  note: string | null;
}

export interface PortfolioHolding {
  product_id: string;
  product_name: string;
  fund_type: string;
  manager_name: string;
  total_units: number;
  avg_buy_nav: number;
  total_invested: number;
  current_nav: number;
  current_value: number;
  unrealized_pnl: number;
  roi_percentage: number;
}

export interface PortfolioSnapshot {
  day: number;
  total_value: number;
  total_invested: number;
  unrealized_pnl: number;
}

export interface Obligation {
  id: string;
  direction: "DEBT" | "RECEIVABLE";
  counterparty: string;
  title: string;
  original_amount: number;
  remaining_amount: number;
  due_date: number;
  note: string;
  status: "OPEN" | "DONE";
  created_at: number;
  updated_at: number;
}

export interface ObligationSummary {
  total_debt: number;
  total_receivable: number;
  overdue_count: number;
}

export interface PortfolioSummary {
  total_value: number;
  total_invested: number;
  unrealized_pnl: number;
}

export const api = {
  accounts_list: () => invoke<Account[]>("accounts_list"),
  accounts_create: (input: Record<string, unknown>) => invoke<Account>("accounts_create", { input }),
  accounts_update: (id: string, input: Record<string, unknown>) => invoke<Account>("accounts_update", { id, input }),
  accounts_delete: (id: string) => invoke<void>("accounts_delete", { id }),

  categories_list: () => invoke<Category[]>("categories_list"),
  categories_create: (input: Record<string, unknown>) => invoke<Category>("categories_create", { input }),
  categories_delete: (id: string) => invoke<void>("categories_delete", { id }),

  transactions_list: (args: Record<string, unknown> = {}) => invoke<Transaction[]>("transactions_list", args),
  transactions_create: (input: Record<string, unknown>) => invoke<Transaction>("transactions_create", { input }),
  transactions_update: (id: string, input: Record<string, unknown>) => invoke<Transaction>("transactions_update", { id, input }),
  transactions_delete: (id: string) => invoke<void>("transactions_delete", { id }),

  pockets_list: () => invoke<SavingsPocket[]>("pockets_list"),
  pockets_create: (input: Record<string, unknown>) => invoke<SavingsPocket>("pockets_create", { input }),
  pockets_update: (id: string, input: Record<string, unknown>) => invoke<SavingsPocket>("pockets_update", { id, input }),
  pockets_delete: (id: string) => invoke<void>("pockets_delete", { id }),
  pockets_deposit: (id: string, amount: number) => invoke<SavingsPocket>("pockets_deposit", { id, amount }),
  pockets_withdraw: (id: string, amount: number) => invoke<SavingsPocket>("pockets_withdraw", { id, amount }),

  dashboard_summary: (from: number, to: number) => invoke<DashboardSummary>("dashboard_summary", { from, to }),
  expense_by_category: (from: number, to: number) => invoke<CategorySpend[]>("expense_by_category", { from, to }),
  export_csv: (from?: number, to?: number) => invoke<string>("export_csv", { from: from ?? null, to: to ?? null }),

  sync_bibit_catalog: () => invoke<number>("sync_bibit_catalog"),
  search_mutual_funds: (query: string, fundType?: string | null) =>
    invoke<MutualFundProduct[]>("search_mutual_funds", { query, fund_type: fundType ?? null }),
  record_investment_tx: (payload: Record<string, unknown>) => invoke<InvestmentTransaction>("record_investment_tx", { payload }),
  get_portfolio_holdings: () => invoke<PortfolioHolding[]>("get_portfolio_holdings"),
  refresh_portfolio_nav: () => invoke<number>("refresh_portfolio_nav"),
  record_daily_snapshot: () => invoke<PortfolioSnapshot>("record_daily_snapshot"),
  get_portfolio_snapshots: (days?: number) =>
    invoke<PortfolioSnapshot[]>("get_portfolio_snapshots", { days: days ?? null }),

  obligations_list: () => invoke<Obligation[]>("obligations_list"),
  obligations_summary: () => invoke<ObligationSummary>("obligations_summary"),
  obligation_create: (input: Record<string, unknown>) => invoke<Obligation>("obligation_create", { input }),
  obligation_update: (id: string, input: Record<string, unknown>) => invoke<Obligation>("obligation_update", { id, input }),
  obligation_delete: (id: string) => invoke<void>("obligation_delete", { id }),
  obligation_pay: (payment: Record<string, unknown>) => invoke<Obligation>("obligation_pay", { payment }),
  portfolio_summary: () => invoke<PortfolioSummary>("portfolio_summary"),

  sync_status: () => invoke<SyncInfo>("sync_status"),
  sync_test: (url: string) => invoke<string>("sync_test", { url }),
  sync_push: () => invoke<{ success: boolean; message: string; pushed: number }>("sync_push"),
  sync_fetch: () => invoke<unknown>("sync_fetch"),
  settings_save: (sheetUrl: string | null, autoSync: boolean) => invoke<void>("settings_save", { sheet_url: sheetUrl, auto_sync: autoSync }),
  reset_data: () => invoke<void>("reset_data"),
};
