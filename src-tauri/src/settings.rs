use crate::types::Settings;
use std::path::Path;

/// アプリケーションルートを取得（セクション11）
pub fn get_app_root() -> std::path::PathBuf {
    if cfg!(debug_assertions) {
        // 開発時: プロジェクトルート
        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
    } else {
        // リリース時: EXEのディレクトリ
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."))
    }
}

/// 設定ファイルを読み込む（セクション7.3）
pub fn load_settings(path: &Path) -> Settings {
    if !path.exists() {
        return Settings::default();
    }

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Settings::default(),
    };

    // デフォルト値とのdeep merge: serdeのデフォルト値機能で実現
    serde_json::from_str(&content).unwrap_or_default()
}

/// 設定ファイルを保存する
pub fn save_settings(path: &Path, settings: &Settings) -> Result<(), String> {
    let json = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("設定のシリアライズに失敗: {}", e))?;
    std::fs::write(path, json).map_err(|e| format!("設定ファイルの保存に失敗: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_nonexistent_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no_such_file.json");
        let settings = load_settings(&path);
        assert!(settings.input_dir.is_empty());
        assert!(settings.is_move_mode); // デフォルトtrue
        assert!(!settings.recursive_scan);
        assert_eq!(settings.dict_path, "folder_dictionary.json");
    }

    #[test]
    fn test_load_invalid_json_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, "not json").unwrap();
        let settings = load_settings(&path);
        assert!(settings.input_dir.is_empty());
    }

    #[test]
    fn test_load_partial_json_merges_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"input_dir": "/some/path"}"#).unwrap();
        let settings = load_settings(&path);
        assert_eq!(settings.input_dir, "/some/path");
        assert!(settings.is_move_mode); // デフォルト値で補完
        assert_eq!(settings.dict_path, "folder_dictionary.json");
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");

        let mut settings = Settings::default();
        settings.input_dir = "C:\\input".to_string();
        settings.output_dir = "C:\\output".to_string();
        settings.recursive_scan = true;
        settings.is_move_mode = false;

        save_settings(&path, &settings).unwrap();
        let loaded = load_settings(&path);
        assert_eq!(loaded.input_dir, "C:\\input");
        assert_eq!(loaded.output_dir, "C:\\output");
        assert!(loaded.recursive_scan);
        assert!(!loaded.is_move_mode);
    }
}
