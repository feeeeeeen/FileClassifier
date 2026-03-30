use crate::dictionary::{parse_main_sub, Dictionary};
use crate::fs_utils::resolve_collision;
use crate::normalize::normalize_for_similarity;
use crate::types::{SimilarCandidate, SimilarGroup, SmallFolder};
use std::collections::{HashSet, VecDeque};
use std::path::Path;

/// 辞書キーの類似マッチング
/// 正規化キーが辞書に完全一致しない場合、辞書の各フォルダ名を
/// Main(Sub)構造で解析し、3.4Aと同じ4条件で類似判定する
pub fn find_similar_dict_match(normalized_key: &str, dict: &Dictionary) -> Option<String> {
    let sim_key = normalize_for_similarity(normalized_key);
    if sim_key.is_empty() {
        return None;
    }

    // 入力タグの解析: (main_key, sub_keys, all_keys)
    let (tag_main, tag_subs, tag_all) = analyze_name(normalized_key);

    // 辞書のフォルダ名（値）をユニークに集め、各フォルダ名をMain(Sub)解析する
    let mut seen_folders = HashSet::new();
    for folder_name in dict.values() {
        if !seen_folders.insert(folder_name.clone()) {
            continue;
        }

        let (fld_main, fld_subs, fld_all) = analyze_name(folder_name);

        // 条件1: メインキー一致
        if !tag_main.is_empty() && tag_main == fld_main {
            return Some(folder_name.clone());
        }

        // 条件2: サブキー完全一致
        if !tag_subs.is_empty() && !fld_subs.is_empty() && tag_subs == fld_subs {
            return Some(folder_name.clone());
        }

        // 条件3: キーセット一致
        if !tag_all.is_empty() && tag_all == fld_all {
            return Some(folder_name.clone());
        }

        // 条件4: 包摂関係
        // タグ（サブキーなし）のメインキーが、フォルダのサブキーに含まれる
        if tag_subs.is_empty() && fld_subs.contains(&tag_main) {
            return Some(folder_name.clone());
        }
        // フォルダ（サブキーなし）のメインキーが、タグのサブキーに含まれる
        if fld_subs.is_empty() && tag_subs.contains(&fld_main) {
            return Some(folder_name.clone());
        }
    }

    // フォルダ名で見つからなかった場合、辞書キー同士の類似比較もフォールバックとして行う
    for (dict_key, folder_name) in dict {
        let dict_sim = normalize_for_similarity(dict_key);
        if dict_sim == sim_key {
            return Some(folder_name.clone());
        }
    }

    None
}

/// 類似フォルダ検出（セクション3.4A）
pub fn detect_similar_folders(
    output_dir: &Path,
    dict: &Dictionary,
) -> Vec<SimilarGroup> {
    // 実フォルダ一覧
    let real_folders: Vec<String> = list_subdirs(output_dir);

    // 辞書のフォルダ名（値の重複除去）
    let dict_folders: HashSet<String> = dict.values().cloned().collect();

    // 全候補を集める（実フォルダ + 辞書名）
    let mut all_names: Vec<(String, bool, bool)> = Vec::new(); // (name, is_real, is_dict)

    for name in &real_folders {
        let is_dict = dict_folders.contains(name);
        all_names.push((name.clone(), true, is_dict));
    }

    for name in &dict_folders {
        if !real_folders.contains(name) {
            all_names.push((name.clone(), false, true));
        }
    }

    // 各候補の類似判定用キーを生成
    let analyzed: Vec<AnalyzedName> = all_names
        .iter()
        .map(|(name, is_real, is_dict)| {
            let (main_key, sub_keys, all_keys) = analyze_name(name);
            AnalyzedName {
                name: name.clone(),
                is_real: *is_real,
                is_dict: *is_dict,
                main_key,
                sub_keys,
                all_keys,
            }
        })
        .collect();

    // 類似ペアを検出
    let n = analyzed.len();
    let mut adjacency: Vec<HashSet<usize>> = vec![HashSet::new(); n];

    for i in 0..n {
        for j in (i + 1)..n {
            // 辞書名同士の場合、少なくとも片方が実フォルダと関連必要
            if !analyzed[i].is_real && !analyzed[j].is_real {
                continue;
            }

            if are_similar(&analyzed[i], &analyzed[j]) {
                adjacency[i].insert(j);
                adjacency[j].insert(i);
            }
        }
    }

    // BFSで推移的クラスタリング
    let mut visited = vec![false; n];
    let mut groups = Vec::new();
    let mut group_id = 0;

    for start in 0..n {
        if visited[start] || adjacency[start].is_empty() {
            continue;
        }

        let mut cluster = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(start);
        visited[start] = true;

        while let Some(current) = queue.pop_front() {
            cluster.push(current);
            for &neighbor in &adjacency[current] {
                if !visited[neighbor] {
                    visited[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }

        if cluster.len() >= 2 {
            let candidates: Vec<SimilarCandidate> = cluster
                .into_iter()
                .map(|idx| SimilarCandidate {
                    name: analyzed[idx].name.clone(),
                    is_real_folder: analyzed[idx].is_real,
                    is_dictionary: analyzed[idx].is_dict,
                })
                .collect();

            groups.push(SimilarGroup {
                id: group_id,
                candidates,
            });
            group_id += 1;
        }
    }

    groups
}

struct AnalyzedName {
    name: String,
    is_real: bool,
    is_dict: bool,
    main_key: String,
    sub_keys: HashSet<String>,
    all_keys: HashSet<String>,
}

fn analyze_name(name: &str) -> (String, HashSet<String>, HashSet<String>) {
    let mut all_keys = HashSet::new();
    let main_key;
    let mut sub_keys = HashSet::new();

    if let Some((main_part, subs)) = parse_main_sub(name) {
        main_key = normalize_for_similarity(&main_part);
        all_keys.insert(main_key.clone());
        for sub in subs {
            let sk = normalize_for_similarity(&sub);
            sub_keys.insert(sk.clone());
            all_keys.insert(sk);
        }
    } else {
        main_key = normalize_for_similarity(name);
        all_keys.insert(main_key.clone());
    }

    (main_key, sub_keys, all_keys)
}

/// 類似判定条件（セクション3.4A.3）
fn are_similar(a: &AnalyzedName, b: &AnalyzedName) -> bool {
    // 条件1: メインキー一致
    if !a.main_key.is_empty() && a.main_key == b.main_key {
        return true;
    }

    // 条件2: サブキー完全一致（両方にサブキーがある場合）
    if !a.sub_keys.is_empty() && !b.sub_keys.is_empty() && a.sub_keys == b.sub_keys {
        return true;
    }

    // 条件3: キーセット一致
    if !a.all_keys.is_empty() && a.all_keys == b.all_keys {
        return true;
    }

    // 条件4: 包摂関係（サブキーなしフォルダのメインキーが他のサブキーに含まれる）
    if a.sub_keys.is_empty() && b.sub_keys.contains(&a.main_key) {
        return true;
    }
    if b.sub_keys.is_empty() && a.sub_keys.contains(&b.main_key) {
        return true;
    }

    false
}

/// 少数ファイルフォルダ検出（セクション3.4B）
pub fn detect_small_folders(output_dir: &Path, threshold: usize) -> Vec<SmallFolder> {
    let subdirs = list_subdirs(output_dir);
    let mut results = Vec::new();

    for name in subdirs {
        let dir_path = output_dir.join(&name);
        let count = count_files_in_dir(&dir_path);
        if count <= threshold {
            results.push(SmallFolder {
                name,
                file_count: count,
            });
        }
    }

    results.sort_by(|a, b| a.file_count.cmp(&b.file_count).then(a.name.cmp(&b.name)));
    results
}

fn list_subdirs(dir: &Path) -> Vec<String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect()
}

fn count_files_in_dir(dir: &Path) -> usize {
    match std::fs::read_dir(dir) {
        Ok(entries) => entries.flatten().filter(|e| e.path().is_file()).count(),
        Err(_) => 0,
    }
}

/// 類似フォルダ統合処理（セクション3.4A.5）
/// 戻り値: Ok((moved_files, warnings)) — warnings はソース読み取り失敗などの部分失敗情報
pub fn merge_folders(
    output_dir: &Path,
    target_name: &str,
    source_names: &[String],
) -> Result<(Vec<String>, Vec<String>), String> {
    let target_path = output_dir.join(target_name);
    std::fs::create_dir_all(&target_path)
        .map_err(|e| format!("統合先フォルダの作成に失敗: {}", e))?;

    let mut moved_files = Vec::new();
    let mut warnings = Vec::new();

    for source_name in source_names {
        if source_name == target_name {
            continue;
        }
        let source_path = output_dir.join(source_name);
        if !source_path.is_dir() {
            continue;
        }

        let entries = match std::fs::read_dir(&source_path) {
            Ok(e) => e,
            Err(e) => {
                let msg = format!("統合元フォルダの読み取りに失敗: {}: {}", source_path.display(), e);
                eprintln!("{}", msg);
                warnings.push(msg);
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let file_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let dest = resolve_collision(&target_path, &file_name)
                .map_err(|e| format!("衝突回避失敗: {}", e))?;
            if let Err(e) = std::fs::rename(&path, &dest) {
                return Err(format!("ファイル移動失敗: {} -> {}: {}",
                    path.display(), dest.display(), e));
            }
            moved_files.push(file_name);
        }

        // 空になったフォルダを削除
        if let Err(e) = std::fs::remove_dir(&source_path) {
            eprintln!("空フォルダの削除に失敗: {}: {}", source_path.display(), e);
        }
    }

    Ok((moved_files, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similar_match_subsumption() {
        // 辞書に Main(Sub) 形式のフォルダがある場合、Subだけのタグで類似マッチする
        let mut dict = Dictionary::new();
        dict.insert("inkcomplex(智弘カイ)".to_string(), "Ink Complex (智弘カイ)".to_string());
        dict.insert("inkcomplex".to_string(), "Ink Complex (智弘カイ)".to_string());
        dict.insert("智弘カイ".to_string(), "Ink Complex (智弘カイ)".to_string());

        // 完全一致キーがある場合はfind_similar_dict_matchの出番なし（classifierで先に解決される）
        // ここでは辞書にキーがない表記ゆれケースをテスト

        // カタカナ→ひらがな変換で類似マッチ（辞書キーとの比較フォールバック）
        let mut dict2 = Dictionary::new();
        dict2.insert("むらのたみ".to_string(), "むらの・たみ".to_string());
        let result = find_similar_dict_match("ムラノタミ", &dict2);
        assert_eq!(result, Some("むらの・たみ".to_string()));
    }

    #[test]
    fn test_similar_match_main_key() {
        // フォルダ名のメインキーとタグのメインキーが類似マッチする
        let mut dict = Dictionary::new();
        dict.insert("somefolder".to_string(), "SomeFolder (SubA)".to_string());

        // "SomeFolder" のタグ → normalize = "somefolder" → 辞書に完全一致キーあり
        // ここでは辞書キーがない場合を想定
        let mut dict2 = Dictionary::new();
        dict2.insert("x".to_string(), "SomeFolder (SubA)".to_string());

        // "SomeFolder" → analyze_name → main="somefolder", subs=[]
        // "SomeFolder (SubA)" → analyze_name → main="somefolder", subs=["suba"]
        // 条件1: メインキー一致 → マッチ
        let result = find_similar_dict_match("somefolder", &dict2);
        assert_eq!(result, Some("SomeFolder (SubA)".to_string()));
    }

    #[test]
    fn test_similar_match_folder_sub_contains_tag() {
        // タグ(サブキーなし)のメインキーが、フォルダのサブキーに含まれる（条件4）
        let mut dict = Dictionary::new();
        dict.insert("x".to_string(), "関西漁業協同組合 (丸新)".to_string());

        // タグ "丸新" → analyze → main="丸新の類似キー", subs=[]
        // フォルダ → main="関西漁業協同組合の類似キー", subs={"丸新の類似キー"}
        // 条件4: tag.subs空 && fld.subsがtag.mainを含む → マッチ
        let result = find_similar_dict_match("丸新", &dict);
        assert_eq!(result, Some("関西漁業協同組合 (丸新)".to_string()));
    }

    #[test]
    fn test_detect_similar_folders_transitivity() {
        // BFSによる推移的クラスタリング: A≈B かつ B≈C → {A, B, C} が同一グループ
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        // 実フォルダを作成: A・B, A-B, A-B(C)
        // A・B と A-B は類似判定用正規化で同一 → 条件1メインキー一致
        // A-B と A-B(C) はメインキー一致 → 推移的に3つとも同一グループ
        std::fs::create_dir(base.join("A・B")).unwrap();
        std::fs::create_dir(base.join("A-B")).unwrap();
        std::fs::create_dir(base.join("A-B(C)")).unwrap();
        // 無関係なフォルダ
        std::fs::create_dir(base.join("XYZ")).unwrap();

        let dict = Dictionary::new();
        let groups = detect_similar_folders(base, &dict);

        // A・B, A-B, A-B(C) が1つのグループにまとまるはず
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].candidates.len(), 3);

        let names: Vec<&str> = groups[0].candidates.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"A・B"));
        assert!(names.contains(&"A-B"));
        assert!(names.contains(&"A-B(C)"));
    }

    #[test]
    fn test_detect_similar_folders_no_false_positives() {
        // 無関係なフォルダ同士はグループにならない
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        std::fs::create_dir(base.join("FolderA")).unwrap();
        std::fs::create_dir(base.join("FolderB")).unwrap();
        std::fs::create_dir(base.join("完全に別")).unwrap();

        let dict = Dictionary::new();
        let groups = detect_similar_folders(base, &dict);
        assert_eq!(groups.len(), 0);
    }
}
