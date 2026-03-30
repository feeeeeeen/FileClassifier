/// ファイル名からタグ名を抽出する（セクション4）
/// 最初の `[` と `]` の間の文字列を取得し、サニタイズする
pub fn extract_tag(filename: &str) -> Option<String> {
    let start = filename.find('[')?;
    let end = filename[start..].find(']')?;
    let raw = filename[start + 1..start + end].trim().to_string();

    if raw.is_empty() {
        return None;
    }

    let sanitized = sanitize_tag(&raw);
    if sanitized.is_empty() {
        None
    } else {
        Some(sanitized)
    }
}

/// タグ名のサニタイズ（セクション4.2）
fn sanitize_tag(tag: &str) -> String {
    // Step 1: パス区切り文字を削除
    let mut result: String = tag
        .chars()
        .filter(|c| *c != '/' && *c != '\\' && *c != ':')
        .collect();

    // Step 2: `..` を繰り返し除去
    while result.contains("..") {
        result = result.replace("..", "");
    }

    // Step 3: 先頭の `.` を除去
    result = result.trim_start_matches('.').to_string();

    result
}

/// ファイル名補正（セクション3.1.3）
pub fn correct_filename(filename: &str, remove_tag: bool, normalize_numbers: bool) -> String {
    let mut result = filename.to_string();

    // 著者名削除: [タグ] 部分を削除し、前後の空白をトリム、連続空白を圧縮
    if remove_tag {
        if let Some(start) = result.find('[') {
            if let Some(end_offset) = result[start..].find(']') {
                let end = start + end_offset + 1;
                result = format!("{}{}", &result[..start], &result[end..]);
                // 連続空白を1つに圧縮
                while result.contains("  ") {
                    result = result.replace("  ", " ");
                }
                result = result.trim().to_string();
            }
        }
    }

    // 数字半角統一: 全角数字→半角（NFKC）+ 数字前後の空白除去
    if normalize_numbers {
        let mut chars: Vec<char> = Vec::new();
        let result_chars: Vec<char> = result.chars().collect();
        for c in result_chars.iter() {
            let normalized = normalize_fullwidth_digit(*c);
            let is_digit = normalized.is_ascii_digit();

            // 数字前の空白除去
            if is_digit && !chars.is_empty() {
                while chars.last().is_some_and(|last| last.is_whitespace()) {
                    chars.pop();
                }
            }

            chars.push(normalized);
        }

        // 数字直後の空白を除去
        let mut final_chars: Vec<char> = Vec::new();
        for (i, c) in chars.iter().enumerate() {
            if c.is_whitespace() && i > 0 && normalize_fullwidth_digit(chars[i - 1]).is_ascii_digit() {
                continue;
            }
            final_chars.push(*c);
        }
        result = final_chars.into_iter().collect();
    }

    result
}

fn normalize_fullwidth_digit(c: char) -> char {
    let code = c as u32;
    // 全角数字 (U+FF10..U+FF19) → 半角 (0x30..0x39)
    if (0xFF10..=0xFF19).contains(&code) {
        char::from_u32(code - 0xFF10 + 0x30).unwrap_or(c)
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tag() {
        assert_eq!(extract_tag("[TagA] file.txt"), Some("TagA".to_string()));
        assert_eq!(extract_tag("prefix [Tag B] file.txt"), Some("Tag B".to_string()));
        assert_eq!(extract_tag("no_tag_file.txt"), None);
        assert_eq!(extract_tag("[] empty.txt"), None);
        assert_eq!(extract_tag("[ ] spaces.txt"), None);
    }

    #[test]
    fn test_sanitize_path_traversal() {
        assert_eq!(extract_tag("[../etc/passwd]"), Some("etcpasswd".to_string()));
        assert_eq!(extract_tag("[foo/bar]"), Some("foobar".to_string()));
        assert_eq!(extract_tag("[.hidden]"), Some("hidden".to_string()));
    }

    #[test]
    fn test_correct_filename_remove_tag() {
        // 著者名削除ON: [Tag]部分を削除
        assert_eq!(
            correct_filename("[Author] file.txt", true, false),
            "file.txt"
        );
        assert_eq!(
            correct_filename("prefix [Tag] file.txt", true, false),
            "prefix file.txt"
        );
        // 著者名削除OFF: そのまま
        assert_eq!(
            correct_filename("[Author] file.txt", false, false),
            "[Author] file.txt"
        );
    }
}
