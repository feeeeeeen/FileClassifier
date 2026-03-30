use unicode_normalization::UnicodeNormalization;

/// 正規化処理（セクション5）
/// Step 1: NFKC正規化
/// Step 2: 小文字化
/// Step 3: 空白除去
/// Step 4: 文字フィルタリング（英数字 + () + , のみ残す）
pub fn normalize(input: &str) -> String {
    let nfkc: String = input.nfkc().collect();
    let lowered = nfkc.to_lowercase();
    let no_spaces: String = lowered.chars().filter(|c| !c.is_whitespace()).collect();
    no_spaces
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '(' || *c == ')' || *c == ',')
        .collect()
}

/// 類似判定用の拡張正規化（セクション3.4A.2）
/// 通常の正規化に加えて:
/// Step 2: 丸括弧・カンマの除去
/// Step 3: カタカナ→ひらがな変換
/// Step 4: 漢数字→アラビア数字変換
pub fn normalize_for_similarity(input: &str) -> String {
    let base = normalize(input);
    let no_parens: String = base
        .chars()
        .filter(|c| *c != '(' && *c != ')' && *c != ',')
        .collect();
    let hiragana: String = no_parens.chars().map(katakana_to_hiragana).collect();
    replace_kanji_numbers(&hiragana)
}

fn katakana_to_hiragana(c: char) -> char {
    let code = c as u32;
    // 全角カタカナ (U+30A1..U+30F6) → ひらがな (U+3041..U+3096)
    if (0x30A1..=0x30F6).contains(&code) {
        char::from_u32(code - 0x60).unwrap_or(c)
    } else {
        c
    }
}

fn replace_kanji_numbers(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '一' => result.push('1'),
            '二' => result.push('2'),
            '三' => result.push('3'),
            '四' => result.push('4'),
            '五' => result.push('5'),
            '六' => result.push('6'),
            '七' => result.push('7'),
            '八' => result.push('8'),
            '九' => result.push('9'),
            '十' => {
                result.push('1');
                result.push('0');
            }
            '百' => {
                result.push('1');
                result.push('0');
                result.push('0');
            }
            '千' => {
                result.push('1');
                result.push('0');
                result.push('0');
                result.push('0');
            }
            '万' => {
                result.push('1');
                result.push('0');
                result.push('0');
                result.push('0');
                result.push('0');
            }
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_basic() {
        assert_eq!(normalize("フォルダ 名A"), "フォルダ名a");
        assert_eq!(normalize("Folder Name A"), "foldernamea");
        assert_eq!(normalize("Folder(B)"), "folder(b)");
        assert_eq!(normalize("！Full Width！"), "fullwidth");
        assert_eq!(normalize("A, B"), "a,b");
    }

    #[test]
    fn test_normalize_nfkc() {
        // 全角英数→半角
        assert_eq!(normalize("Ａ１Ｂ"), "a1b");
    }

    #[test]
    fn test_similarity_normalization() {
        assert_eq!(normalize_for_similarity("A・B"), normalize_for_similarity("A-B"));
        assert_eq!(normalize_for_similarity("むらの・たみ"), normalize_for_similarity("むらのたみ"));
        assert_eq!(normalize_for_similarity("ムラノタミ"), normalize_for_similarity("むらのたみ"));
        assert_eq!(normalize_for_similarity("第一工房"), normalize_for_similarity("第1工房"));
        assert_eq!(normalize_for_similarity("Ａ１Ｂ"), normalize_for_similarity("A1B"));
    }
}
