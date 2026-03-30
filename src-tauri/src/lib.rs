pub mod classifier;
pub mod commands;
pub mod dictionary;
pub mod fs_utils;
pub mod normalize;
pub mod settings;
pub mod similarity;
pub mod tag;
pub mod types;

use commands::*;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_root = settings::get_app_root();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState {
            dict: std::sync::Mutex::new(dictionary::Dictionary::new()),
            logs: std::sync::Mutex::new(Vec::new()),
            cancel_flag: std::sync::atomic::AtomicBool::new(false),
            app_root,
        })
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            load_dictionary,
            save_dictionary,
            run_dry_run,
            run_classify,
            cancel_classify,
            create_dictionary_from_folder,
            update_dictionary_entry,
            add_dictionary_entry,
            remove_dictionary_key,
            remove_dictionary_folder,
            detect_similar_folders,
            merge_similar_folders,
            detect_small_folders,
            merge_small_folders,
            get_logs,
            clear_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
