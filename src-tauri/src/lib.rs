mod bibit;
mod commands;
mod db;
mod models;
mod sync;

use tauri::Manager;

use db::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Auto-update: updater fetches latest.json from GitHub Releases;
        // process plugin supplies relaunch() after install.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let state = AppState::new(&app.handle())?;
            app.manage(state);
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                loop {
                    commands::daily_portfolio_job(handle.clone()).await;
                    tokio::time::sleep(std::time::Duration::from_secs(24 * 60 * 60)).await;
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::accounts_list,
            commands::accounts_create,
            commands::accounts_update,
            commands::accounts_delete,
            commands::categories_list,
            commands::categories_create,
            commands::categories_delete,
            commands::transactions_list,
            commands::transactions_create,
            commands::transactions_update,
            commands::transactions_delete,
            commands::pockets_list,
            commands::pockets_create,
            commands::pockets_update,
            commands::pockets_delete,
            commands::pockets_deposit,
            commands::pockets_withdraw,
            commands::dashboard_summary,
            commands::expense_by_category,
            commands::export_csv,
            commands::sync_bibit_catalog,
            commands::search_mutual_funds,
            commands::record_investment_tx,
            commands::get_portfolio_holdings,
            commands::refresh_portfolio_nav,
            commands::record_daily_snapshot,
            commands::get_portfolio_snapshots,
            commands::sync_status,
            commands::sync_test,
            commands::sync_fetch,
            commands::sync_push,
            commands::settings_save,
            commands::reset_data,
            commands::obligations_list,
            commands::obligations_summary,
            commands::obligation_create,
            commands::obligation_update,
            commands::obligation_delete,
            commands::obligation_pay,
            commands::portfolio_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
