use std::path::{Path, PathBuf};

const COLLISION_LIMIT: u32 = 10_000;

/// フォルダ名のサニタイズ（パストラバーサル防止）
/// パス区切り文字、`..`、先頭の`.`を除去する
pub fn sanitize_folder_name(name: &str) -> String {
    let mut result: String = name
        .chars()
        .filter(|c| *c != '/' && *c != '\\' && *c != ':')
        .collect();
    while result.contains("..") {
        result = result.replace("..", "");
    }
    result = result.trim_start_matches('.').to_string();
    result
}

/// ファイル名衝突回避（セクション10）
/// 同名ファイルが存在する場合 _1, _2, ... サフィックスを付与
pub fn resolve_collision(dir: &Path, filename: &str) -> Result<PathBuf, String> {
    let dest = dir.join(filename);
    if !dest.exists() {
        return Ok(dest);
    }

    let stem = Path::new(filename)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let ext = Path::new(filename)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    for counter in 1..=COLLISION_LIMIT {
        let new_name = format!("{}_{}{}", stem, counter, ext);
        let new_path = dir.join(&new_name);
        if !new_path.exists() {
            return Ok(new_path);
        }
    }

    Err(format!(
        "ファイル名の衝突回避に失敗: '{}' (上限{}に到達)",
        filename, COLLISION_LIMIT
    ))
}

/// 出力パスが基準ディレクトリ内に留まるか検証
/// targetがまだ存在しない場合でも正しく判定する
pub fn validate_path_within_dir(base: &Path, target: &Path) -> Result<(), String> {
    // baseをcanonicalizeして確定パスを取得
    let canonical_base = base
        .canonicalize()
        .map_err(|e| format!("基準パスの正規化に失敗: {}", e))?;

    // targetからbaseへの相対パスを取得し、`..`が含まれないことを確認
    // targetをstrip_prefixでbaseからの相対パスとして取得
    if let Ok(relative) = target.strip_prefix(base) {
        // 相対パスに .. が含まれていないか確認
        for component in relative.components() {
            if let std::path::Component::ParentDir = component {
                return Err(format!(
                    "不正なパスが検出されました: '{}' は '{}' の外部です",
                    target.display(),
                    base.display()
                ));
            }
        }
        // canonical_base + relative で構築されるパスは基準内
        let _ = canonical_base; // baseが実在することは確認済み
        return Ok(());
    }

    // strip_prefixが失敗 = targetがbaseの配下でない
    // targetがcanonicalizeできる場合（既存パス）は再チェック
    if let Ok(canonical_target) = target.canonicalize() {
        if canonical_target.starts_with(&canonical_base) {
            return Ok(());
        }
    }

    Err(format!(
        "不正なパスが検出されました: '{}' は '{}' の外部です",
        target.display(),
        base.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_resolve_collision() {
        let dir = tempdir().unwrap();
        let base = dir.path();

        let result = resolve_collision(base, "test.txt").unwrap();
        assert_eq!(result, base.join("test.txt"));

        fs::write(base.join("test.txt"), "").unwrap();
        let result = resolve_collision(base, "test.txt").unwrap();
        assert_eq!(result, base.join("test_1.txt"));

        fs::write(base.join("test_1.txt"), "").unwrap();
        let result = resolve_collision(base, "test.txt").unwrap();
        assert_eq!(result, base.join("test_2.txt"));
    }

    #[test]
    fn test_validate_path_within_dir() {
        let dir = tempdir().unwrap();
        let base = dir.path();

        // 正常ケース
        assert!(validate_path_within_dir(base, &base.join("subfolder")).is_ok());
        assert!(validate_path_within_dir(base, &base.join("sub/nested")).is_ok());
    }
}
