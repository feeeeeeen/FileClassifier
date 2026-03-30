export interface Settings {
  input_dir: string;
  output_dir: string;
  dict_path: string;
  is_move_mode: boolean;
  recursive_scan: boolean;
  options: ClassifyOptions;
}

export interface ClassifyOptions {
  remove_tag: boolean;
  normalize_numbers: boolean;
}

export const defaultSettings: Settings = {
  input_dir: "",
  output_dir: "",
  dict_path: "folder_dictionary.json",
  is_move_mode: true,
  recursive_scan: false,
  options: {
    remove_tag: false,
    normalize_numbers: false,
  },
};

export type MatchType = "dict_exact" | "dict_similar" | "auto_created";

export interface DryRunResult {
  file_name: string;
  file_path: string;
  destination: string | null;
  tag: string | null;
  status: "classifiable" | "unclassifiable";
  match_type: MatchType | null;
}

export interface DestinationOverride {
  file_path: string;
  original_destination: string;
  new_destination: string;
  tag: string;
  match_type: MatchType;
}

export interface ClassifyProgress {
  current: number;
  total: number;
  file_name: string;
}

export interface ClassifySummary {
  success: number;
  skipped: number;
  errors: number;
}

export interface LogEntry {
  timestamp: string;
  operation: string;
  source: string;
  destination: string;
  action: string;
  detail: string;
}

export interface SimilarGroup {
  id: number;
  candidates: SimilarCandidate[];
}

export interface SimilarCandidate {
  name: string;
  is_real_folder: boolean;
  is_dictionary: boolean;
}

export interface SmallFolder {
  name: string;
  file_count: number;
}

export interface MergeGroup {
  target_name: string;
  source_names: string[];
}

export interface DictionaryData {
  entries: DictionaryGroup[];
}

export interface DictionaryGroup {
  folder_name: string;
  keys: string[];
}
