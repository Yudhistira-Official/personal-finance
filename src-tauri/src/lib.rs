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
        .setup(|app| {
            let state = AppState::new(&app.handle())?;
            app.manage(state);
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
            commands::sync_status,
            commands::sync_test,
            commands::sync_fetch,
            commands::sync_push,
            commands::settings_save,
            commands::reset_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
