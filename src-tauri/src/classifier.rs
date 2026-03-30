use crate::dictionary::Dictionary;
use crate::fs_utils::{resolve_collision, validate_path_within_dir};
use crate::normalize::normalize;
use crate::similarity::find_similar_dict_match;
use crate::tag::{correct_filename, extract_tag};
use crate::types::{ClassifyOptions, DryRunResult, FileStatus, LogEntry, MatchType};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// ファイル一覧を取得する（セクション3.1.4）
pub fn scan_files(input_dir: &Path, recursive: bool, output_dir: Option<&Path>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    scan_dir(input_dir, recursive, output_dir, &mut files);
    files
}

fn scan_dir(
    dir: &Path,
    recursive: bool,
    output_dir: Option<&Path>,
    files: &mut Vec<PathBuf>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("ディレクトリの読み取りに失敗: {}: {}", dir.display(), e);
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        } else if path.is_dir() && recursive {
            // 出力フォルダ自体はスキャン対象から除外
            if let Some(out_dir) = output_dir {
                if path.starts_with(out_dir) {
                    continue;
                }
            }
            scan_dir(&path, recursive, output_dir, files);
        }
    }
}

/// タグから分類先フォルダを解決する
/// 戻り値: (フォルダ名, マッチ種別)
fn resolve_destination(tag_name: &str, dict: &Dictionary) -> (String, MatchType) {
    let key = normalize(tag_name);

    // 1. 辞書完全一致
    if let Some(folder) = dict.get(&key) {
        return (folder.clone(), MatchType::DictExact);
    }

    // 2. 類似マッチ
    if let Some(folder) = find_similar_dict_match(&key, dict) {
        return (folder, MatchType::DictSimilar);
    }

    // 3. 自動作成（タグ名をそのままフォルダ名に）
    (tag_name.to_string(), MatchType::AutoCreated)
}

/// ドライラン実行（F-05）
pub fn dry_run(
    input_dir: &Path,
    output_dir: &Path,
    dict: &Dictionary,
    recursive: bool,
) -> Vec<DryRunResult> {
    let output_path = if output_dir.as_os_str().is_empty() {
        None
    } else {
        Some(output_dir)
    };
    let files = scan_files(input_dir, recursive, output_path);
    let mut results = Vec::new();

    for file_path in &files {
        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let tag = extract_tag(&file_name);

        match &tag {
            None => {
                results.push(DryRunResult {
                    file_name,
                    file_path: file_path.to_string_lossy().to_string(),
                    destination: None,
                    tag: None,
                    status: FileStatus::Unclassifiable,
                    match_type: None,
                });
            }
            Some(tag_name) => {
                let (destination, mt) = resolve_destination(tag_name, dict);
                results.push(DryRunResult {
                    file_name,
                    file_path: file_path.to_string_lossy().to_string(),
                    destination: Some(destination),
                    tag: Some(tag_name.clone()),
                    status: FileStatus::Classifiable,
                    match_type: Some(mt),
                });
            }
        }
    }

    results
}

/// ファイル分類実行（F-01）
pub fn classify_files(
    input_dir: &Path,
    output_dir: &Path,
    dict: &mut Dictionary,
    options: &ClassifyOptions,
    is_move: bool,
    recursive: bool,
    cancel_flag: &std::sync::atomic::AtomicBool,
    progress_callback: impl Fn(usize, usize, &str),
    overrides: &HashMap<String, String>,
) -> (Vec<LogEntry>, HashMap<String, String>) {
    let files = scan_files(input_dir, recursive, Some(output_dir));
    let total = files.len();
    let mut logs = Vec::new();
    let mut new_entries = HashMap::new();
    let timestamp = chrono_now();

    for (i, file_path) in files.iter().enumerate() {
        // 中断チェック（F-07）
        if cancel_flag.load(std::sync::atomic::Ordering::Relaxed) {
            logs.push(LogEntry {
                timestamp: timestamp.clone(),
                operation: "classify".to_string(),
                source: String::new(),
                destination: String::new(),
                action: "cancelled".to_string(),
                detail: format!("{}件目で中断しました", i),
            });
            break;
        }

        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        progress_callback(i, total, &file_name);

        let tag = match extract_tag(&file_name) {
            Some(t) => t,
            None => {
                logs.push(LogEntry {
                    timestamp: timestamp.clone(),
                    operation: "classify".to_string(),
                    source: file_path.to_string_lossy().to_string(),
                    destination: String::new(),
                    action: "skip".to_string(),
                    detail: "タグなし".to_string(),
                });
                continue;
            }
        };

        let file_path_str = file_path.to_string_lossy().to_string();
        let key = normalize(&tag);

        // overridesがあればそれを優先、なければ辞書解決
        let folder_name = if let Some(overridden) = overrides.get(&file_path_str) {
            overridden.clone()
        } else if let Some(folder) = dict.get(&key) {
            folder.clone()
        } else if let Some(folder) = find_similar_dict_match(&key, dict) {
            dict.insert(key.clone(), folder.clone());
            new_entries.insert(key.clone(), folder.clone());
            folder
        } else {
            dict.insert(key.clone(), tag.clone());
            new_entries.insert(key.clone(), tag.clone());
            tag.clone()
        };

        let dest_dir = output_dir.join(&folder_name);

        // パス検証: 出力ディレクトリ内に留まるか確認
        if let Err(e) = validate_path_within_dir(output_dir, &dest_dir) {
            logs.push(LogEntry {
                timestamp: timestamp.clone(),
                operation: "classify".to_string(),
                source: file_path.to_string_lossy().to_string(),
                destination: dest_dir.to_string_lossy().to_string(),
                action: "error".to_string(),
                detail: e,
            });
            continue;
        }

        if let Err(e) = std::fs::create_dir_all(&dest_dir) {
            logs.push(LogEntry {
                timestamp: timestamp.clone(),
                operation: "classify".to_string(),
                source: file_path.to_string_lossy().to_string(),
                destination: dest_dir.to_string_lossy().to_string(),
                action: "error".to_string(),
                detail: format!("フォルダ作成失敗: {}", e),
            });
            continue;
        }

        // ファイル名補正
        let corrected_name = correct_filename(
            &file_name,
            options.remove_tag,
            options.normalize_numbers,
        );

        // ファイル名衝突回避（セクション10）
        let dest_path = match resolve_collision(&dest_dir, &corrected_name) {
            Ok(p) => p,
            Err(e) => {
                logs.push(LogEntry {
                    timestamp: timestamp.clone(),
                    operation: "classify".to_string(),
                    source: file_path.to_string_lossy().to_string(),
                    destination: dest_dir.to_string_lossy().to_string(),
                    action: "error".to_string(),
                    detail: e,
                });
                continue;
            }
        };

        let action_str = if is_move { "move" } else { "copy" };
        let result = if is_move {
            std::fs::rename(file_path, &dest_path)
        } else {
            std::fs::copy(file_path, &dest_path).map(|_| ())
        };

        match result {
            Ok(_) => {
                logs.push(LogEntry {
                    timestamp: timestamp.clone(),
                    operation: "classify".to_string(),
                    source: file_path.to_string_lossy().to_string(),
                    destination: dest_path.to_string_lossy().to_string(),
                    action: action_str.to_string(),
                    detail: String::new(),
                });
            }
            Err(e) => {
                logs.push(LogEntry {
                    timestamp: timestamp.clone(),
                    operation: "classify".to_string(),
                    source: file_path.to_string_lossy().to_string(),
                    destination: dest_path.to_string_lossy().to_string(),
                    action: "error".to_string(),
                    detail: format!("ファイル操作失敗: {}", e),
                });
            }
        }
    }

    (logs, new_entries)
}

fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use tempfile::tempdir;

    fn make_test_file(dir: &std::path::Path, name: &str) {
        fs::write(dir.join(name), "test content").unwrap();
    }

    #[test]
    fn test_dry_run_classifiable() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input");
        let output = dir.path().join("output");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&output).unwrap();

        make_test_file(&input, "[Author] file.txt");
        make_test_file(&input, "no_tag.txt");

        let mut dict = Dictionary::new();
        dict.insert("author".to_string(), "Author".to_string());

        let results = dry_run(&input, &output, &dict, false);
        assert_eq!(results.len(), 2);

        let classifiable = results.iter().find(|r| r.file_name == "[Author] file.txt").unwrap();
        assert_eq!(classifiable.status, FileStatus::Classifiable);
        assert_eq!(classifiable.destination.as_deref(), Some("Author"));

        let skipped = results.iter().find(|r| r.file_name == "no_tag.txt").unwrap();
        assert_eq!(skipped.status, FileStatus::Unclassifiable);
    }

    #[test]
    fn test_dry_run_similar_match() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input");
        let output = dir.path().join("output");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&output).unwrap();

        make_test_file(&input, "[ムラノタミ] file.txt");

        let mut dict = Dictionary::new();
        dict.insert("むらのたみ".to_string(), "むらの・たみ".to_string());

        let results = dry_run(&input, &output, &dict, false);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, FileStatus::Classifiable);
        assert_eq!(results[0].destination.as_deref(), Some("むらの・たみ"));
    }

    #[test]
    fn test_dry_run_auto_create() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input");
        let output = dir.path().join("output");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&output).unwrap();

        make_test_file(&input, "[NewTag] file.txt");

        let dict = Dictionary::new();
        let results = dry_run(&input, &output, &dict, false);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, FileStatus::Classifiable);
        assert_eq!(results[0].destination.as_deref(), Some("NewTag"));
    }

    #[test]
    fn test_classify_files_copy_mode() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input");
        let output = dir.path().join("output");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&output).unwrap();

        make_test_file(&input, "[Author] file.txt");

        let mut dict = Dictionary::new();
        dict.insert("author".to_string(), "Author".to_string());
        let cancel = AtomicBool::new(false);
        let options = ClassifyOptions { remove_tag: false, normalize_numbers: false };

        let (logs, _) = classify_files(
            &input, &output, &mut dict, &options,
            false, false, &cancel, |_, _, _| {},
            &HashMap::new(),
        );

        assert!(logs.iter().any(|l| l.action == "copy"));
        assert!(output.join("Author").join("[Author] file.txt").exists());
        // コピーモードなので元ファイルも残る
        assert!(input.join("[Author] file.txt").exists());
    }

    #[test]
    fn test_classify_files_move_mode() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input");
        let output = dir.path().join("output");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&output).unwrap();

        make_test_file(&input, "[Author] file.txt");

        let mut dict = Dictionary::new();
        dict.insert("author".to_string(), "Author".to_string());
        let cancel = AtomicBool::new(false);
        let options = ClassifyOptions { remove_tag: false, normalize_numbers: false };

        let (logs, _) = classify_files(
            &input, &output, &mut dict, &options,
            true, false, &cancel, |_, _, _| {},
            &HashMap::new(),
        );

        assert!(logs.iter().any(|l| l.action == "move"));
        assert!(output.join("Author").join("[Author] file.txt").exists());
        // 移動モードなので元ファイルは消える
        assert!(!input.join("[Author] file.txt").exists());
    }

    #[test]
    fn test_classify_files_cancel() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input");
        let output = dir.path().join("output");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&output).unwrap();

        for i in 0..5 {
            make_test_file(&input, &format!("[Tag] file{}.txt", i));
        }

        let mut dict = Dictionary::new();
        dict.insert("tag".to_string(), "Tag".to_string());
        let cancel = AtomicBool::new(true); // 最初からキャンセル
        let options = ClassifyOptions { remove_tag: false, normalize_numbers: false };

        let (logs, _) = classify_files(
            &input, &output, &mut dict, &options,
            false, false, &cancel, |_, _, _| {},
            &HashMap::new(),
        );

        assert!(logs.iter().any(|l| l.action == "cancelled"));
    }

    #[test]
    fn test_classify_files_auto_register() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input");
        let output = dir.path().join("output");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&output).unwrap();

        make_test_file(&input, "[NewAuthor] file.txt");

        let mut dict = Dictionary::new();
        let cancel = AtomicBool::new(false);
        let options = ClassifyOptions { remove_tag: false, normalize_numbers: false };

        let (_, new_entries) = classify_files(
            &input, &output, &mut dict, &options,
            false, false, &cancel, |_, _, _| {},
            &HashMap::new(),
        );

        // 辞書に自動登録される
        assert!(dict.contains_key("newauthor"));
        assert!(new_entries.contains_key("newauthor"));
        assert!(output.join("NewAuthor").join("[NewAuthor] file.txt").exists());
    }

    #[test]
    fn test_classify_files_collision() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input");
        let output = dir.path().join("output");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&output).unwrap();

        make_test_file(&input, "[Tag] file.txt");
        // 出力先に同名ファイルを事前配置
        let dest_dir = output.join("Tag");
        fs::create_dir_all(&dest_dir).unwrap();
        fs::write(dest_dir.join("[Tag] file.txt"), "existing").unwrap();

        let mut dict = Dictionary::new();
        dict.insert("tag".to_string(), "Tag".to_string());
        let cancel = AtomicBool::new(false);
        let options = ClassifyOptions { remove_tag: false, normalize_numbers: false };

        classify_files(
            &input, &output, &mut dict, &options,
            false, false, &cancel, |_, _, _| {},
            &HashMap::new(),
        );

        // 衝突回避で _1 サフィックスが付く
        assert!(dest_dir.join("[Tag] file_1.txt").exists());
    }

    #[test]
    fn test_classify_with_overrides() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input");
        let output = dir.path().join("output");
        fs::create_dir_all(&input).unwrap();
        fs::create_dir_all(&output).unwrap();

        make_test_file(&input, "[Tag] file.txt");

        let mut dict = Dictionary::new();
        let cancel = AtomicBool::new(false);
        let options = ClassifyOptions { remove_tag: false, normalize_numbers: false };

        // overridesで分類先を「CustomFolder」に変更
        let file_path = input.join("[Tag] file.txt").to_string_lossy().to_string();
        let mut overrides = HashMap::new();
        overrides.insert(file_path, "CustomFolder".to_string());

        let (logs, _) = classify_files(
            &input, &output, &mut dict, &options,
            false, false, &cancel, |_, _, _| {},
            &overrides,
        );

        assert!(logs.iter().any(|l| l.action == "copy"));
        assert!(output.join("CustomFolder").join("[Tag] file.txt").exists());
    }

    #[test]
    fn test_recursive_scan() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("input");
        let sub = input.join("sub");
        fs::create_dir_all(&sub).unwrap();

        make_test_file(&input, "[A] top.txt");
        make_test_file(&sub, "[A] nested.txt");

        let files_flat = scan_files(&input, false, None);
        assert_eq!(files_flat.len(), 1);

        let files_recursive = scan_files(&input, true, None);
        assert_eq!(files_recursive.len(), 2);
    }
}

