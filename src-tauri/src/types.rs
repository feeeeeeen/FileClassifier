use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub input_dir: String,
    #[serde(default)]
    pub output_dir: String,
    #[serde(default = "default_dict_path")]
    pub dict_path: String,
    #[serde(default = "default_true")]
    pub is_move_mode: bool,
    #[serde(default)]
    pub recursive_scan: bool,
    #[serde(default)]
    pub options: ClassifyOptions,
}

fn default_true() -> bool {
    true
}

fn default_dict_path() -> String {
    "folder_dictionary.json".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            input_dir: String::new(),
            output_dir: String::new(),
            dict_path: default_dict_path(),
            is_move_mode: true,
            recursive_scan: false,
            options: ClassifyOptions::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClassifyOptions {
    #[serde(default)]
    pub remove_tag: bool,
    #[serde(default)]
    pub normalize_numbers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunResult {
    pub file_name: String,
    pub file_path: String,
    pub destination: Option<String>,
    pub tag: Option<String>,
    pub status: FileStatus,
    pub match_type: Option<MatchType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    /// 辞書一致（完全一致 or 類似マッチ or 自動作成）
    Classifiable,
    /// タグなし・タグ無効で分類不可
    Unclassifiable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MatchType {
    DictExact,
    DictSimilar,
    AutoCreated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestinationOverride {
    pub file_path: String,
    pub original_destination: String,
    pub new_destination: String,
    pub tag: String,
    pub match_type: MatchType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyProgress {
    pub current: usize,
    pub total: usize,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub operation: String,
    pub source: String,
    pub destination: String,
    pub action: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarGroup {
    pub id: usize,
    pub candidates: Vec<SimilarCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarCandidate {
    pub name: String,
    pub is_real_folder: bool,
    pub is_dictionary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmallFolder {
    pub name: String,
    pub file_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeGroup {
    pub target_name: String,
    pub source_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryData {
    pub entries: Vec<DictionaryGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryGroup {
    pub folder_name: String,
    pub keys: Vec<String>,
}
