use crate::classifier;
use crate::dictionary;
use crate::normalize::normalize;
use crate::settings;
use crate::similarity;
use crate::types::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

pub struct AppState {
    pub dict: Mutex<dictionary::Dictionary>,
    pub logs: Mutex<Vec<LogEntry>>,
    pub cancel_flag: AtomicBool,
    pub app_root: PathBuf,
}

impl AppState {
    pub fn dict_path(&self, settings: &Settings) -> PathBuf {
        self.app_root.join(&settings.dict_path)
    }

    fn lock_dict(&self) -> Result<std::sync::MutexGuard<'_, dictionary::Dictionary>, String> {
        self.dict
            .lock()
            .map_err(|e| format!("辞書のロック取得に失敗: {}", e))
    }

    fn lock_logs(&self) -> Result<std::sync::MutexGuard<'_, Vec<LogEntry>>, String> {
        self.logs
            .lock()
            .map_err(|e| format!("ログのロック取得に失敗: {}", e))
    }
}

fn get_or_init_state(app: &tauri::AppHandle) -> &AppState {
    // Tauriのstate管理を使用
    app.state::<AppState>().inner()
}

#[tauri::command]
pub fn load_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let state = get_or_init_state(&app);
    let settings_path = state.app_root.join("settings.json");
    let settings = settings::load_settings(&settings_path);

    // 辞書も読み込む
    let dict_path = state.dict_path(&settings);
    match dictionary::load_dictionary(&dict_path) {
        Ok(dict) => {
            *state.lock_dict()? = dict;
        }
        Err(errors) => {
            return Err(errors.join("\n"));
        }
    }

    Ok(settings)
}

#[tauri::command]
pub fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    let state = get_or_init_state(&app);
    let settings_path = state.app_root.join("settings.json");
    settings::save_settings(&settings_path, &settings)
}

#[tauri::command]
pub fn load_dictionary(app: tauri::AppHandle) -> Result<DictionaryData, String> {
    let state = get_or_init_state(&app);
    let dict = state.lock_dict()?;
    let groups = dictionary::dictionary_to_groups(&dict);
    Ok(DictionaryData { entries: groups })
}

#[tauri::command]
pub fn save_dictionary(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    let state = get_or_init_state(&app);
    let dict = state.lock_dict()?;
    let dict_path = state.dict_path(&settings);
    dictionary::save_dictionary(&dict_path, &dict)
}

#[tauri::command]
pub fn run_dry_run(
    app: tauri::AppHandle,
    settings: Settings,
) -> Result<Vec<DryRunResult>, String> {
    let state = get_or_init_state(&app);
    let input_dir = PathBuf::from(&settings.input_dir);
    let output_dir = PathBuf::from(&settings.output_dir);

    if !input_dir.is_dir() {
        return Err("入力フォルダが存在しません".to_string());
    }

    let is_same_dir = input_dir.canonicalize().ok()
        .zip(output_dir.canonicalize().ok())
        .is_some_and(|(ic, oc)| ic == oc);
    let recursive = if is_same_dir { false } else { settings.recursive_scan };

    let dict = state.lock_dict()?;
    Ok(classifier::dry_run(
        &input_dir,
        &output_dir,
        &dict,
        recursive,
    ))
}

#[tauri::command]
pub async fn run_classify(
    app: tauri::AppHandle,
    settings: Settings,
    overrides: Vec<DestinationOverride>,
) -> Result<ClassifySummary, String> {
    let state = get_or_init_state(&app);
    let input_dir = PathBuf::from(&settings.input_dir);
    let output_dir = PathBuf::from(&settings.output_dir);

    if !input_dir.is_dir() {
        return Err("入力フォルダが存在しません".to_string());
    }
    if settings.output_dir.is_empty() {
        return Err("出力フォルダが指定されていません".to_string());
    }
    let is_same_dir = input_dir.canonicalize().ok()
        .zip(output_dir.canonicalize().ok())
        .is_some_and(|(ic, oc)| ic == oc);
    let recursive = if is_same_dir { false } else { settings.recursive_scan };

    state.cancel_flag.store(false, Ordering::Relaxed);

    let app_handle = app.clone();
    let cancel_flag = &state.cancel_flag;

    let logs = {
        let mut dict = state.lock_dict()?;

        // overridesによる辞書更新（分類前に実行）
        for ov in &overrides {
            if ov.original_destination == ov.new_destination {
                continue;
            }
            // new_destination をサニタイズ（パストラバーサル防止）
            let new_dest = crate::fs_utils::sanitize_folder_name(&ov.new_destination);
            if new_dest.is_empty() {
                continue;
            }
            match ov.match_type {
                MatchType::AutoCreated => {
                    let tag_key = normalize(&ov.tag);
                    if !tag_key.is_empty() {
                        dict.insert(tag_key, new_dest.clone());
                    }
                    for (k, v) in dictionary::generate_keys_from_folder_name(&new_dest) {
                        dict.insert(k, v);
                    }
                }
                MatchType::DictExact | MatchType::DictSimilar => {
                    for value in dict.values_mut() {
                        if *value == ov.original_destination {
                            *value = new_dest.clone();
                        }
                    }
                    for (k, _) in dictionary::generate_keys_from_folder_name(&ov.original_destination) {
                        dict.insert(k, new_dest.clone());
                    }
                    for (k, v) in dictionary::generate_keys_from_folder_name(&new_dest) {
                        dict.insert(k, v);
                    }
                }
            }
        }

        // overridesマップ (file_path → new_destination) を構築
        let override_map: std::collections::HashMap<String, String> = overrides
            .iter()
            .filter(|ov| ov.original_destination != ov.new_destination)
            .map(|ov| {
                let sanitized = crate::fs_utils::sanitize_folder_name(&ov.new_destination);
                (ov.file_path.clone(), sanitized)
            })
            .filter(|(_, dest)| !dest.is_empty())
            .collect();

        let (logs, new_entries) = classifier::classify_files(
            &input_dir,
            &output_dir,
            &mut dict,
            &settings.options,
            settings.is_move_mode,
            recursive,
            cancel_flag,
            |current, total, file_name| {
                let _ = app_handle.emit(
                    "classify-progress",
                    ClassifyProgress {
                        current,
                        total,
                        file_name: file_name.to_string(),
                    },
                );
            },
            &override_map,
        );

        // 辞書保存（同一ロックスコープ内で実行）
        if !new_entries.is_empty() || !override_map.is_empty() {
            let dict_path = state.dict_path(&settings);
            dictionary::save_dictionary(&dict_path, &dict)?;
        }

        logs
    };

    // ログ記録
    let success = logs.iter().filter(|l| l.action == "copy" || l.action == "move").count();
    let skipped = logs.iter().filter(|l| l.action == "skip").count();
    let errors = logs.iter().filter(|l| l.action == "error").count();

    state.lock_logs()?.extend(logs);

    Ok(ClassifySummary {
        success,
        skipped,
        errors,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassifySummary {
    pub success: usize,
    pub skipped: usize,
    pub errors: usize,
}

#[tauri::command]
pub fn cancel_classify(app: tauri::AppHandle) {
    let state = get_or_init_state(&app);
    state.cancel_flag.store(true, Ordering::Relaxed);
}

#[tauri::command]
pub fn create_dictionary_from_folder(
    app: tauri::AppHandle,
    folder_path: String,
    settings: Settings,
) -> Result<DictionaryData, String> {
    let state = get_or_init_state(&app);
    let path = PathBuf::from(&folder_path);

    if !path.is_dir() {
        return Err("指定されたフォルダが存在しません".to_string());
    }

    let mut new_dict = dictionary::Dictionary::new();

    let entries = std::fs::read_dir(&path)
        .map_err(|e| format!("フォルダの読み込みに失敗: {}", e))?;

    for entry in entries.flatten() {
        if entry.path().is_dir() {
            let folder_name = entry.file_name().to_string_lossy().to_string();
            let keys = dictionary::generate_keys_from_folder_name(&folder_name);
            for (key, value) in keys {
                new_dict.insert(key, value);
            }
        }
    }

    *state.lock_dict()? = new_dict.clone();

    // 保存
    let dict_path = state.dict_path(&settings);
    dictionary::save_dictionary(&dict_path, &new_dict)?;

    let groups = dictionary::dictionary_to_groups(&new_dict);
    Ok(DictionaryData { entries: groups })
}

#[tauri::command]
pub fn update_dictionary_entry(
    app: tauri::AppHandle,
    old_folder_name: String,
    new_folder_name: String,
) -> Result<(), String> {
    let state = get_or_init_state(&app);
    let mut dict = state.lock_dict()?;

    // 旧フォルダ名に関連する全キーの値を更新
    for value in dict.values_mut() {
        if *value == old_folder_name {
            *value = new_folder_name.clone();
        }
    }

    Ok(())
}

#[tauri::command]
pub fn add_dictionary_entry(
    app: tauri::AppHandle,
    folder_name: String,
    key: Option<String>,
) -> Result<String, String> {
    let state = get_or_init_state(&app);
    let mut dict = state.lock_dict()?;

    if let Some(raw_key) = key {
        // 特定キーを追加
        let normalized = normalize(&raw_key);
        if normalized.is_empty() {
            return Err("正規化後のキーが空です".to_string());
        }
        if let Some(existing) = dict.get(&normalized) {
            if *existing != folder_name {
                return Err(format!(
                    "キー '{}' は既に '{}' に登録されています",
                    normalized, existing
                ));
            }
        }
        dict.insert(normalized.clone(), folder_name);
        Ok(normalized)
    } else {
        // フォルダ名からキーを自動生成
        if folder_name.is_empty() {
            return Err("フォルダ名が空です".to_string());
        }
        if dict.values().any(|v| v == &folder_name) {
            return Err(format!("フォルダ '{}' は既に存在します", folder_name));
        }

        let keys = dictionary::generate_keys_from_folder_name(&folder_name);
        for (k, v) in &keys {
            if let Some(existing) = dict.get(k) {
                if *existing != *v {
                    return Err(format!(
                        "キー '{}' は既に '{}' に登録されています",
                        k, existing
                    ));
                }
            }
        }

        for (k, v) in keys {
            dict.insert(k, v);
        }

        Ok(folder_name)
    }
}

#[tauri::command]
pub fn remove_dictionary_key(app: tauri::AppHandle, key: String) -> Result<(), String> {
    let state = get_or_init_state(&app);
    let mut dict = state.lock_dict()?;
    let normalized = normalize(&key);
    dict.remove(&normalized);
    Ok(())
}

#[tauri::command]
pub fn remove_dictionary_folder(app: tauri::AppHandle, folder_name: String) -> Result<(), String> {
    let state = get_or_init_state(&app);
    let mut dict = state.lock_dict()?;
    dict.retain(|_, v| *v != folder_name);
    Ok(())
}


#[tauri::command]
pub fn detect_similar_folders(
    app: tauri::AppHandle,
    settings: Settings,
) -> Result<Vec<SimilarGroup>, String> {
    let state = get_or_init_state(&app);
    let output_dir = PathBuf::from(&settings.output_dir);
    if !output_dir.is_dir() {
        return Err("出力フォルダが存在しません".to_string());
    }
    let dict = state.lock_dict()?;
    Ok(similarity::detect_similar_folders(&output_dir, &dict))
}

#[tauri::command]
pub fn merge_similar_folders(
    app: tauri::AppHandle,
    settings: Settings,
    groups: Vec<MergeGroup>,
) -> Result<Vec<String>, String> {
    let state = get_or_init_state(&app);
    let output_dir = PathBuf::from(&settings.output_dir);
    let dict_path = state.dict_path(&settings);

    // 辞書のバックアップを保持（FS操作失敗時のロールバック用）
    let dict_backup = state.lock_dict()?.clone();

    // 辞書を先に更新・保存（FS操作失敗時はロールバック）
    {
        let mut dict = state.lock_dict()?;
        for group in &groups {
            let keys = dictionary::generate_keys_from_folder_name(&group.target_name);
            for (k, v) in keys {
                dict.insert(k, v);
            }
            for source in &group.source_names {
                let source_keys = dictionary::generate_keys_from_folder_name(source);
                for (k, _) in source_keys {
                    dict.insert(k, group.target_name.clone());
                }
            }
        }
        dictionary::save_dictionary(&dict_path, &dict)?;
    }

    // FS操作を実行
    let mut all_warnings: Vec<String> = Vec::new();
    for group in &groups {
        match similarity::merge_folders(&output_dir, &group.target_name, &group.source_names) {
            Ok((_moved, warnings)) => {
                all_warnings.extend(warnings);
            }
            Err(e) => {
                // FS操作失敗時は辞書をロールバック
                let mut dict = state.lock_dict()?;
                *dict = dict_backup;
                if let Err(re) = dictionary::save_dictionary(&dict_path, &dict) {
                    eprintln!("ロールバック中の辞書保存に失敗: {}", re);
                }
                return Err(e);
            }
        }
    }

    Ok(all_warnings)
}

#[tauri::command]
pub fn detect_small_folders(
    _app: tauri::AppHandle,
    settings: Settings,
    threshold: usize,
) -> Result<Vec<SmallFolder>, String> {
    let output_dir = PathBuf::from(&settings.output_dir);
    if !output_dir.is_dir() {
        return Err("出力フォルダが存在しません".to_string());
    }
    Ok(similarity::detect_small_folders(&output_dir, threshold))
}

#[tauri::command]
pub fn merge_small_folders(
    app: tauri::AppHandle,
    settings: Settings,
    groups: Vec<MergeGroup>,
) -> Result<Vec<String>, String> {
    merge_similar_folders(app, settings, groups)
}

#[tauri::command]
pub fn get_logs(app: tauri::AppHandle) -> Result<Vec<LogEntry>, String> {
    let state = get_or_init_state(&app);
    Ok(state.lock_logs()?.clone())
}

#[tauri::command]
pub fn clear_logs(app: tauri::AppHandle) -> Result<(), String> {
    let state = get_or_init_state(&app);
    state.lock_logs()?.clear();
    Ok(())
}
