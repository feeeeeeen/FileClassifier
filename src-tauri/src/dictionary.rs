use crate::normalize::normalize;
use crate::types::DictionaryGroup;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

/// 辞書: 正規化キー → 分類先フォルダ名
pub type Dictionary = HashMap<String, String>;

/// 辞書ファイルを読み込む（F-08: フォーマットチェック付き）
pub fn load_dictionary(path: &Path) -> Result<Dictionary, Vec<String>> {
    let mut warnings = Vec::new();

    if !path.exists() {
        return Ok(Dictionary::new());
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| vec![format!("辞書ファイルの読み込みに失敗しました: {}", e)])?;

    let value: Value = serde_json::from_str(&content)
        .map_err(|e| vec![format!("JSONのパースに失敗しました: {}", e)])?;

    let obj = value
        .as_object()
        .ok_or_else(|| vec!["辞書ファイルのトップレベルがオブジェクトではありません".to_string()])?;

    let mut dict = Dictionary::new();

    for (key, val) in obj {
        match val.as_str() {
            Some(folder_name) => {
                let normalized = normalize(key);
                if normalized != *key {
                    warnings.push(format!(
                        "キー '{}' は正規化ルールに準拠していません（正規化後: '{}'）",
                        key, normalized
                    ));
                }
                // 正規化したキーで保存する
                dict.insert(normalized, folder_name.to_string());
            }
            None => {
                warnings.push(format!(
                    "キー '{}' の値が文字列型ではありません。スキップします",
                    key
                ));
            }
        }
    }

    if !warnings.is_empty() {
        // 警告はあるが読み込みは継続
        eprintln!("辞書の警告: {:?}", warnings);
    }

    Ok(dict)
}

/// 辞書ファイルを保存する
pub fn save_dictionary(path: &Path, dict: &Dictionary) -> Result<(), String> {
    // キーをソートして保存
    let mut sorted: Vec<_> = dict.iter().collect();
    sorted.sort_by_key(|(k, _)| (*k).clone());

    let map: serde_json::Map<String, Value> = sorted
        .into_iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();

    let json = serde_json::to_string_pretty(&Value::Object(map))
        .map_err(|e| format!("JSONシリアライズに失敗: {}", e))?;

    std::fs::write(path, json).map_err(|e| format!("辞書ファイルの保存に失敗: {}", e))
}

/// フォルダ名から辞書キーを生成するグループ化ルール（セクション6.2）
/// Main(Sub1, Sub2) 形式のフォルダ名から複数のキーを生成する
pub fn generate_keys_from_folder_name(folder_name: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();

    // フルネームのキー
    let full_key = normalize(folder_name);
    if !full_key.is_empty() {
        entries.push((full_key, folder_name.to_string()));
    }

    // Main(Sub) のパース
    if let Some((main_part, subs)) = parse_main_sub(folder_name) {
        let main_key = normalize(&main_part);
        if !main_key.is_empty() {
            entries.push((main_key, folder_name.to_string()));
        }
        for sub in subs {
            let sub_key = normalize(&sub);
            if !sub_key.is_empty() {
                entries.push((sub_key, folder_name.to_string()));
            }
        }
    }

    entries
}

/// Main(Sub1, Sub2) 形式をパースする
pub fn parse_main_sub(name: &str) -> Option<(String, Vec<String>)> {
    // 正規表現 ^(.*?)\((.*)\)$ に相当
    let trimmed = name.trim();
    let paren_start = trimmed.find('(')?;
    if !trimmed.ends_with(')') {
        return None;
    }

    let main_part = trimmed[..paren_start].trim().to_string();
    let inside = &trimmed[paren_start + 1..trimmed.len() - 1];
    let subs: Vec<String> = inside
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if main_part.is_empty() && subs.is_empty() {
        return None;
    }

    Some((main_part, subs))
}

/// 辞書をグループ化して表示用データに変換する
pub fn dictionary_to_groups(dict: &Dictionary) -> Vec<DictionaryGroup> {
    let mut folder_map: HashMap<String, Vec<String>> = HashMap::new();

    for (key, folder) in dict {
        folder_map.entry(folder.clone()).or_default().push(key.clone());
    }

    let mut groups: Vec<DictionaryGroup> = folder_map
        .into_iter()
        .map(|(folder_name, mut keys)| {
            keys.sort();
            DictionaryGroup { folder_name, keys }
        })
        .collect();

    groups.sort_by(|a, b| a.folder_name.cmp(&b.folder_name));
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_main_sub() {
        let (main, subs) = parse_main_sub("Ink Complex (智弘カイ)").unwrap();
        assert_eq!(main, "Ink Complex");
        assert_eq!(subs, vec!["智弘カイ"]);

        let (main, subs) = parse_main_sub("関西漁業協同組合 (丸新)").unwrap();
        assert_eq!(main, "関西漁業協同組合");
        assert_eq!(subs, vec!["丸新"]);

        assert!(parse_main_sub("SimpleFolder").is_none());
    }

    #[test]
    fn test_generate_keys() {
        let keys = generate_keys_from_folder_name("Ink Complex (智弘カイ)");
        assert!(keys.len() >= 3); // full, main, sub
    }

    #[test]
    fn test_load_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dict.json");

        // 不正なJSON
        std::fs::write(&path, "not json").unwrap();
        let result = load_dictionary(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_non_object_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dict.json");

        // トップレベルが配列
        std::fs::write(&path, "[1,2,3]").unwrap();
        let result = load_dictionary(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_non_string_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dict.json");

        // 非文字列値はスキップされる
        std::fs::write(&path, r#"{"key1": "Folder", "key2": 123}"#).unwrap();
        let dict = load_dictionary(&path).unwrap();
        assert_eq!(dict.len(), 1);
        assert_eq!(dict.get("key1").unwrap(), "Folder");
    }

    #[test]
    fn test_load_normalizes_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dict.json");

        // 非正規化キーは正規化して読み込まれる
        std::fs::write(&path, r#"{"Folder Name A": "FolderA"}"#).unwrap();
        let dict = load_dictionary(&path).unwrap();
        assert!(dict.contains_key("foldernamea"));
        assert!(!dict.contains_key("Folder Name A"));
    }

    #[test]
    fn test_load_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dict.json");

        // 空オブジェクト
        std::fs::write(&path, "{}").unwrap();
        let dict = load_dictionary(&path).unwrap();
        assert!(dict.is_empty());
    }

    #[test]
    fn test_load_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no_such_file.json");
        let dict = load_dictionary(&path).unwrap();
        assert!(dict.is_empty());
    }

    #[test]
    fn test_dictionary_to_groups() {
        let mut dict = Dictionary::new();
        dict.insert("key1".to_string(), "FolderA".to_string());
        dict.insert("key2".to_string(), "FolderA".to_string());
        dict.insert("key3".to_string(), "FolderB".to_string());

        let groups = dictionary_to_groups(&dict);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].folder_name, "FolderA");
        assert_eq!(groups[0].keys.len(), 2);
    }
}
