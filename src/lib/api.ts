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

  sync_status: () => invoke<SyncInfo>("sync_status"),
  sync_test: (url: string) => invoke<string>("sync_test", { url }),
  sync_push: () => invoke<{ success: boolean; message: string; pushed: number }>("sync_push"),
  sync_fetch: () => invoke<unknown>("sync_fetch"),
  settings_save: (sheetUrl: string | null, autoSync: boolean) => invoke<void>("settings_save", { sheet_url: sheetUrl, auto_sync: autoSync }),
  reset_data: () => invoke<void>("reset_data"),
};
