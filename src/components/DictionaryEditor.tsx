import { useState, useEffect, useCallback, useRef } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import * as api from "../lib/api";
import type { DictionaryGroup, Settings } from "../lib/types";
import { defaultSettings } from "../lib/types";

interface Props {
  onBack: () => void;
}

export default function DictionaryEditor({ onBack }: Props) {
  const [groups, setGroups] = useState<DictionaryGroup[]>([]);
  const [settings, setSettings] = useState<Settings>(defaultSettings);
  const [searchQuery, setSearchQuery] = useState("");
  const [editingFolder, setEditingFolder] = useState<string | null>(null);
  const [editingKey, setEditingKey] = useState<{
    folder: string;
    key: string;
  } | null>(null);
  const [addingKeyFolder, setAddingKeyFolder] = useState<string | null>(null);
  const [addingFolder, setAddingFolder] = useState(false);
  const [newFolderName, setNewFolderName] = useState("");
  const [newKeyValue, setNewKeyValue] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const tableRef = useRef<HTMLDivElement>(null);

  // 初回読み込み（設定+辞書をディスクから）
  const initLoad = useCallback(async () => {
    try {
      const s = await api.loadSettings();
      setSettings(s);
      const dict = await api.loadDictionary();
      setGroups(dict.entries);
    } catch (e) {
      setError(`読み込みエラー: ${e}`);
    } finally {
      setLoading(false);
    }
  }, []);

  // 辞書のみメモリから再取得（設定リロードしない）
  const refreshDict = useCallback(async () => {
    try {
      const dict = await api.loadDictionary();
      setGroups(dict.entries);
    } catch (e) {
      setError(`${e}`);
    }
  }, []);

  useEffect(() => {
    initLoad();
  }, [initLoad]);

  // HOME/ENDキー
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (!tableRef.current) return;
      if (
        e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement
      )
        return;
      if (e.key === "Home") {
        tableRef.current.scrollTop = 0;
      } else if (e.key === "End") {
        tableRef.current.scrollTop = tableRef.current.scrollHeight;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  const scrollToBottom = () => {
    requestAnimationFrame(() => {
      if (tableRef.current) {
        tableRef.current.scrollTop = tableRef.current.scrollHeight;
      }
    });
  };

  const handleSave = async () => {
    try {
      await api.saveDictionary(settings);
      setError("saved");
    } catch (e) {
      setError(`保存エラー: ${e}`);
    }
  };

  const handleAddFolder = async () => {
    if (!newFolderName.trim()) {
      setAddingFolder(false);
      setNewFolderName("");
      return;
    }
    try {
      await api.addDictionaryEntry(newFolderName.trim());
      setNewFolderName("");
      setAddingFolder(false);
      setError(null);
      await refreshDict();
      scrollToBottom();
    } catch (e) {
      setError(`${e}`);
    }
  };

  const handleStartAddFolder = () => {
    setAddingFolder(true);
    setNewFolderName("");
    scrollToBottom();
  };

  const handleAddKey = async (folderName: string) => {
    if (!newKeyValue.trim()) return;
    try {
      await api.addDictionaryEntry(folderName, newKeyValue.trim());
      setNewKeyValue("");
      setError(null);
      setAddingKeyFolder(null);
      await refreshDict();
    } catch (e) {
      setError(`${e}`);
    }
  };

  const handleRemoveKey = async (key: string) => {
    try {
      await api.removeDictionaryKey(key);
      await refreshDict();
    } catch (e) {
      setError(`${e}`);
    }
  };

  const handleRemoveFolder = async (folderName: string) => {
    if (
      !confirm(
        `「${folderName}」グループを削除しますか？\n配下の全キーも削除されます。`,
      )
    )
      return;
    try {
      await api.removeDictionaryFolder(folderName);
      await refreshDict();
    } catch (e) {
      setError(`${e}`);
    }
  };

  const handleRenameFolderCommit = async (
    oldName: string,
    newName: string,
  ) => {
    if (newName === oldName || !newName.trim()) {
      setEditingFolder(null);
      return;
    }
    try {
      await api.updateDictionaryEntry(oldName, newName.trim());
      setEditingFolder(null);
      await refreshDict();
    } catch (e) {
      setError(`${e}`);
    }
  };

  const handleRenameKeyCommit = async (
    folder: string,
    oldKey: string,
    newKey: string,
  ) => {
    if (newKey === oldKey || !newKey.trim()) {
      setEditingKey(null);
      return;
    }
    try {
      await api.removeDictionaryKey(oldKey);
      await api.addDictionaryEntry(folder, newKey.trim());
      setEditingKey(null);
      await refreshDict();
    } catch (e) {
      setError(`${e}`);
    }
  };

  const handleCreateDict = async () => {
    const selected = await open({
      directory: true,
      defaultPath: settings.output_dir || undefined,
    });
    if (!selected) return;
    if (!confirm("既存の辞書を上書きしますか？")) return;
    try {
      const dict = await api.createDictionaryFromFolder(selected, settings);
      setGroups(dict.entries);
      setError(null);
    } catch (e) {
      setError(`辞書作成エラー: ${e}`);
    }
  };

  const filteredGroups = searchQuery
    ? groups.filter((g) =>
        g.folder_name.toLowerCase().includes(searchQuery.toLowerCase()),
      )
    : groups;

  return (
    <div className="flex flex-col h-screen">
      {/* ツールバー */}
      <div className="border-b p-3 flex items-center gap-2 shrink-0">
        <button
          className="px-3 py-1 text-sm border rounded hover:bg-gray-100"
          onClick={onBack}
        >
          ← 戻る
        </button>
        <h2 className="text-sm font-bold">辞書編集</h2>
        <div className="flex-1" />
        <input
          type="text"
          placeholder="フォルダ名で検索..."
          className="border rounded px-2 py-1 text-sm w-48"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
        />
        <button
          className="px-3 py-1 text-sm border rounded hover:bg-gray-100"
          onClick={handleCreateDict}
        >
          辞書作成
        </button>
        <button
          className="px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
          onClick={handleSave}
        >
          保存
        </button>
      </div>

      {/* メッセージ表示 */}
      {error && (
        <div className={`border-b px-4 py-2 text-sm flex items-center justify-between ${
          error === "saved"
            ? "bg-green-50 border-green-200 text-green-700"
            : "bg-red-50 border-red-200 text-red-700"
        }`}>
          <span>{error === "saved" ? "辞書を保存しました" : error}</span>
          <button className="ml-2 underline" onClick={() => setError(null)}>
            閉じる
          </button>
        </div>
      )}

      {/* テーブル */}
      <div className="flex-1 overflow-auto" ref={tableRef}>
        {loading ? (
          <p className="text-sm text-gray-500 text-center py-8">
            読み込み中...
          </p>
        ) : filteredGroups.length === 0 && !addingFolder ? (
          <p className="text-sm text-gray-500 text-center py-8">
            辞書が空です。「フォルダ追加」または「辞書作成」で始めてください。
          </p>
        ) : (
          <table className="w-full text-sm border-collapse">
            <thead className="sticky top-0 bg-gray-50 z-10">
              <tr>
                <th className="text-left px-3 py-1.5 border-b font-medium text-gray-700 w-2/5">
                  出力フォルダ
                </th>
                <th className="text-left px-3 py-1.5 border-b font-medium text-gray-700">
                  分類キー
                </th>
                <th className="w-16 border-b" />
              </tr>
            </thead>
            <tbody>
              {filteredGroups.map((group) => (
                <tr
                  key={group.folder_name}
                  className="border-b border-gray-100 hover:bg-gray-50/50 align-top"
                >
                  {/* フォルダ名セル */}
                  <td className="px-3 py-1.5 border-r border-gray-100">
                    {editingFolder === group.folder_name ? (
                      <input
                        className="border rounded px-1 py-0.5 text-sm w-full"
                        defaultValue={group.folder_name}
                        autoFocus
                        onBlur={(e) =>
                          handleRenameFolderCommit(
                            group.folder_name,
                            e.target.value,
                          )
                        }
                        onKeyDown={(e) => {
                          if (e.key === "Enter")
                            handleRenameFolderCommit(
                              group.folder_name,
                              (e.target as HTMLInputElement).value,
                            );
                          if (e.key === "Escape") setEditingFolder(null);
                        }}
                      />
                    ) : (
                      <span
                        className="cursor-pointer hover:text-blue-700"
                        onDoubleClick={() =>
                          setEditingFolder(group.folder_name)
                        }
                        title="ダブルクリックで編集"
                      >
                        {group.folder_name}
                      </span>
                    )}
                  </td>

                  {/* キーセル */}
                  <td className="px-3 py-1">
                    <div className="flex flex-wrap gap-1 items-center">
                      {group.keys.map((key) => (
                        <span
                          key={key}
                          className="inline-flex items-center gap-0.5 bg-gray-100 rounded px-1.5 py-0.5 text-xs"
                        >
                          {editingKey?.folder === group.folder_name &&
                          editingKey?.key === key ? (
                            <input
                              className="border rounded px-1 py-0 text-xs w-24 bg-white"
                              defaultValue={key}
                              autoFocus
                              onBlur={(e) =>
                                handleRenameKeyCommit(
                                  group.folder_name,
                                  key,
                                  e.target.value,
                                )
                              }
                              onKeyDown={(e) => {
                                if (e.key === "Enter")
                                  handleRenameKeyCommit(
                                    group.folder_name,
                                    key,
                                    (e.target as HTMLInputElement).value,
                                  );
                                if (e.key === "Escape") setEditingKey(null);
                              }}
                            />
                          ) : (
                            <span
                              className="cursor-pointer"
                              onDoubleClick={() =>
                                setEditingKey({
                                  folder: group.folder_name,
                                  key,
                                })
                              }
                              title="ダブルクリックで編集"
                            >
                              {key}
                            </span>
                          )}
                          <button
                            className="text-gray-400 hover:text-red-600 leading-none"
                            onClick={() => handleRemoveKey(key)}
                          >
                            ×
                          </button>
                        </span>
                      ))}

                      {/* キー追加 */}
                      {addingKeyFolder === group.folder_name ? (
                        <input
                          className="border rounded px-1.5 py-0.5 text-xs w-28"
                          placeholder="キー入力..."
                          autoFocus
                          value={newKeyValue}
                          onChange={(e) => setNewKeyValue(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter")
                              handleAddKey(group.folder_name);
                            if (e.key === "Escape") {
                              setAddingKeyFolder(null);
                              setNewKeyValue("");
                            }
                          }}
                          onBlur={() => {
                            if (!newKeyValue.trim()) {
                              setAddingKeyFolder(null);
                              setNewKeyValue("");
                            }
                          }}
                        />
                      ) : (
                        <button
                          className="text-xs text-blue-600 hover:text-blue-800 px-1"
                          onClick={() => {
                            setAddingKeyFolder(group.folder_name);
                            setNewKeyValue("");
                          }}
                        >
                          +
                        </button>
                      )}
                    </div>
                  </td>

                  {/* 操作セル */}
                  <td className="px-2 py-1.5 text-center">
                    <button
                      className="text-xs text-red-400 hover:text-red-700"
                      onClick={() => handleRemoveFolder(group.folder_name)}
                    >
                      削除
                    </button>
                  </td>
                </tr>
              ))}

              {/* フォルダ追加行 */}
              {addingFolder && (
                <tr className="border-b border-gray-100 bg-blue-50/30">
                  <td className="px-3 py-1.5 border-r border-gray-100">
                    <input
                      className="border rounded px-1 py-0.5 text-sm w-full"
                      placeholder="フォルダ名を入力..."
                      autoFocus
                      value={newFolderName}
                      onChange={(e) => setNewFolderName(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") handleAddFolder();
                        if (e.key === "Escape") {
                          setAddingFolder(false);
                          setNewFolderName("");
                        }
                      }}
                      onBlur={() => {
                        if (!newFolderName.trim()) {
                          setAddingFolder(false);
                          setNewFolderName("");
                        }
                      }}
                    />
                  </td>
                  <td className="px-3 py-1.5 text-xs text-gray-400">
                    Enterで追加 / Escでキャンセル
                  </td>
                  <td />
                </tr>
              )}
            </tbody>
          </table>
        )}
      </div>

      {/* フッター */}
      <div className="border-t p-2 px-3 flex items-center gap-2 shrink-0">
        <button
          className="px-3 py-1 text-sm border rounded hover:bg-gray-100"
          onClick={handleStartAddFolder}
        >
          フォルダ追加
        </button>
        <span className="text-xs text-gray-400">
          {groups.length}件のフォルダ
        </span>
      </div>
    </div>
  );
}
