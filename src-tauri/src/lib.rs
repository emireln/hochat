mod commands;
mod db;
mod llm;
mod rag;
mod settings;

use commands::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;

            let connection = db::open(&directory.join("hochat.db"))?;
            app.manage(AppState::new(connection));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_chats,
            commands::create_chat,
            commands::rename_chat,
            commands::delete_chat,
            commands::list_messages,
            commands::send_message,
            commands::stop_generation,
            commands::list_models,
            commands::check_provider,
            commands::get_settings,
            commands::save_settings,
            commands::rag_status,
            commands::list_documents,
            commands::delete_document,
            commands::ingest_file,
            commands::storage_info,
            commands::open_data_folder,
            commands::clear_all_chats,
        ])
        .run(tauri::generate_context!())
        .expect("não foi possível iniciar o HoChat");
}
