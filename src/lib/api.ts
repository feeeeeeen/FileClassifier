import { invoke } from "@tauri-apps/api/core";
import type {
  Settings,
  DryRunResult,
  ClassifySummary,
  DestinationOverride,
  DictionaryData,
  SimilarGroup,
  SmallFolder,
  MergeGroup,
  LogEntry,
} from "./types";

export async function loadSettings(): Promise<Settings> {
  return invoke("load_settings");
}

export async function saveSettings(settings: Settings): Promise<void> {
  return invoke("save_settings", { settings });
}

export async function loadDictionary(): Promise<DictionaryData> {
  return invoke("load_dictionary");
}

export async function saveDictionary(settings: Settings): Promise<void> {
  return invoke("save_dictionary", { settings });
}

export async function runDryRun(settings: Settings): Promise<DryRunResult[]> {
  return invoke("run_dry_run", { settings });
}

export async function runClassify(
  settings: Settings,
  overrides: DestinationOverride[] = [],
): Promise<ClassifySummary> {
  return invoke("run_classify", { settings, overrides });
}

export async function cancelClassify(): Promise<void> {
  return invoke("cancel_classify");
}

export async function createDictionaryFromFolder(
  folderPath: string,
  settings: Settings,
): Promise<DictionaryData> {
  return invoke("create_dictionary_from_folder", {
    folderPath,
    settings,
  });
}

export async function updateDictionaryEntry(
  oldFolderName: string,
  newFolderName: string,
): Promise<void> {
  return invoke("update_dictionary_entry", { oldFolderName, newFolderName });
}

export async function addDictionaryEntry(
  folderName: string,
  key?: string,
): Promise<string> {
  return invoke("add_dictionary_entry", { folderName, key: key ?? null });
}

export async function removeDictionaryKey(key: string): Promise<void> {
  return invoke("remove_dictionary_key", { key });
}

export async function removeDictionaryFolder(
  folderName: string,
): Promise<void> {
  return invoke("remove_dictionary_folder", { folderName });
}

export async function detectSimilarFolders(
  settings: Settings,
): Promise<SimilarGroup[]> {
  return invoke("detect_similar_folders", { settings });
}

export async function mergeSimilarFolders(
  settings: Settings,
  groups: MergeGroup[],
): Promise<string[]> {
  return invoke("merge_similar_folders", { settings, groups });
}

export async function detectSmallFolders(
  settings: Settings,
  threshold: number,
): Promise<SmallFolder[]> {
  return invoke("detect_small_folders", { settings, threshold });
}

export async function mergeSmallFolders(
  settings: Settings,
  groups: MergeGroup[],
): Promise<string[]> {
  return invoke("merge_small_folders", { settings, groups });
}

export async function getLogs(): Promise<LogEntry[]> {
  return invoke("get_logs");
}

export async function clearLogs(): Promise<void> {
  return invoke("clear_logs");
}
