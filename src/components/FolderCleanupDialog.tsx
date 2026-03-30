import { useState, useRef, useEffect } from "react";
import * as api from "../lib/api";
import type {
  Settings,
  SimilarGroup,
  SmallFolder,
  MergeGroup,
} from "../lib/types";

interface Props {
  settings: Settings;
  onClose: () => void;
}

type Mode = "similar" | "small";

function InlineMessage({
  message,
  type,
  onDismiss,
}: {
  message: string;
  type: "success" | "error" | "warning";
  onDismiss: () => void;
}) {
  const colors = {
    success: "bg-green-50 border-green-200 text-green-800",
    error: "bg-red-50 border-red-200 text-red-800",
    warning: "bg-yellow-50 border-yellow-200 text-yellow-800",
  };
  return (
    <div className={`border rounded px-3 py-2 mb-3 text-sm flex items-center justify-between ${colors[type]}`}>
      <span>{message}</span>
      <button className="ml-2 opacity-60 hover:opacity-100" onClick={onDismiss}>×</button>
    </div>
  );
}

export default function FolderCleanupDialog({ settings, onClose }: Props) {
  const [mode, setMode] = useState<Mode>("similar");

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow-xl w-[700px] max-h-[80vh] flex flex-col">
        <div className="p-4 border-b shrink-0">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-sm font-bold">出力フォルダ整理</h2>
            <button
              className="text-gray-400 hover:text-gray-700 text-lg leading-none px-1"
              onClick={onClose}
            >
              ×
            </button>
          </div>
          <div className="flex gap-4 text-sm">
            <label className="flex items-center gap-1">
              <input
                type="radio"
                checked={mode === "similar"}
                onChange={() => setMode("similar")}
              />
              類似フォルダを統合する
            </label>
            <label className="flex items-center gap-1">
              <input
                type="radio"
                checked={mode === "small"}
                onChange={() => setMode("small")}
              />
              ファイル数の少ないフォルダを整理する
            </label>
          </div>
        </div>

        <div className="flex-1 overflow-auto">
          {mode === "similar" ? (
            <SimilarMergePanel settings={settings} onClose={onClose} />
          ) : (
            <SmallFolderPanel settings={settings} onClose={onClose} />
          )}
        </div>
      </div>
    </div>
  );
}

function SimilarMergePanel({
  settings,
  onClose,
}: {
  settings: Settings;
  onClose: () => void;
}) {
  const [groups, setGroups] = useState<SimilarGroup[]>([]);
  const [selections, setSelections] = useState<Record<number, string | null>>({});
  const [loading, setLoading] = useState(false);
  const [detected, setDetected] = useState(false);
  const [message, setMessage] = useState<{ text: string; type: "success" | "error" | "warning" } | null>(null);
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (closeTimerRef.current !== null) {
        clearTimeout(closeTimerRef.current);
      }
    };
  }, []);

  const detect = async () => {
    setLoading(true);
    setMessage(null);
    try {
      const result = await api.detectSimilarFolders(settings);
      setGroups(result);
      const defaults: Record<number, string | null> = {};
      for (const g of result) {
        const dictCandidate = g.candidates.find(
          (c) => c.is_dictionary && !c.is_real_folder,
        );
        defaults[g.id] = dictCandidate?.name ?? g.candidates[0]?.name ?? null;
      }
      setSelections(defaults);
      setDetected(true);
    } catch (e) {
      setMessage({ text: `検出エラー: ${e}`, type: "error" });
    }
    setLoading(false);
  };

  const handleMerge = async () => {
    const mergeGroups: MergeGroup[] = [];
    for (const group of groups) {
      const target = selections[group.id];
      if (!target) continue;
      const sourceNames = group.candidates
        .map((c) => c.name)
        .filter((n) => n !== target);
      if (sourceNames.length > 0) {
        mergeGroups.push({ target_name: target, source_names: sourceNames });
      }
    }

    if (mergeGroups.length === 0) {
      setMessage({ text: "統合対象がありません", type: "warning" });
      return;
    }

    try {
      const warnings = await api.mergeSimilarFolders(settings, mergeGroups);
      if (warnings.length > 0) {
        setMessage({ text: `統合完了（警告: ${warnings.join(", ")}）`, type: "warning" });
      } else {
        setMessage({ text: "統合が完了しました", type: "success" });
        closeTimerRef.current = setTimeout(onClose, 1000);
      }
    } catch (e) {
      setMessage({ text: `統合エラー: ${e}`, type: "error" });
    }
  };

  return (
    <div className="p-4">
      {message && (
        <InlineMessage message={message.text} type={message.type} onDismiss={() => setMessage(null)} />
      )}

      {!detected ? (
        <div className="text-center py-8">
          <button
            className="px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
            onClick={detect}
            disabled={loading}
          >
            {loading ? "検出中..." : "類似フォルダを検出"}
          </button>
        </div>
      ) : groups.length === 0 ? (
        <p className="text-center text-gray-500 py-8 text-sm">
          類似フォルダは見つかりませんでした
        </p>
      ) : (
        <>
          <div className="space-y-4 mb-4">
            {groups.map((group) => (
              <div key={group.id} className="border rounded p-3">
                <p className="text-xs text-gray-500 mb-2">
                  グループ{group.id + 1}:
                </p>
                <div className="space-y-1">
                  {group.candidates.map((candidate) => (
                    <label
                      key={candidate.name}
                      className="flex items-center gap-2 text-sm"
                    >
                      <input
                        type="radio"
                        name={`group-${group.id}`}
                        checked={selections[group.id] === candidate.name}
                        onChange={() =>
                          setSelections((p) => ({
                            ...p,
                            [group.id]: candidate.name,
                          }))
                        }
                      />
                      <span
                        className={
                          candidate.is_dictionary && !candidate.is_real_folder
                            ? "font-bold text-blue-700"
                            : ""
                        }
                      >
                        {candidate.name}
                      </span>
                      <span className="text-xs text-gray-400">
                        {candidate.is_real_folder && "(実フォルダ)"}
                        {candidate.is_dictionary &&
                          !candidate.is_real_folder &&
                          "(辞書)"}
                        {candidate.is_dictionary &&
                          !candidate.is_real_folder &&
                          " ← 推奨"}
                      </span>
                    </label>
                  ))}
                  <label className="flex items-center gap-2 text-sm text-gray-500">
                    <input
                      type="radio"
                      name={`group-${group.id}`}
                      checked={selections[group.id] === null}
                      onChange={() =>
                        setSelections((p) => ({ ...p, [group.id]: null }))
                      }
                    />
                    統合しない
                  </label>
                </div>
              </div>
            ))}
          </div>

          <div className="flex justify-end gap-2 border-t pt-3">
            <button
              className="px-4 py-1.5 text-sm border rounded hover:bg-gray-100"
              onClick={onClose}
            >
              キャンセル
            </button>
            <button
              className="px-4 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
              onClick={handleMerge}
            >
              実行
            </button>
          </div>
        </>
      )}
    </div>
  );
}

function SmallFolderPanel({
  settings,
  onClose,
}: {
  settings: Settings;
  onClose: () => void;
}) {
  const [threshold, setThreshold] = useState(3);
  const [folders, setFolders] = useState<SmallFolder[]>([]);
  const [detected, setDetected] = useState(false);
  const [loading, setLoading] = useState(false);
  const [mergeGroups, setMergeGroups] = useState<
    { name: string; items: SmallFolder[] }[]
  >([]);
  const [unassigned, setUnassigned] = useState<SmallFolder[]>([]);
  const [message, setMessage] = useState<{ text: string; type: "success" | "error" | "warning" } | null>(null);
  const closeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (closeTimerRef.current !== null) {
        clearTimeout(closeTimerRef.current);
      }
    };
  }, []);

  const detect = async () => {
    setLoading(true);
    setMessage(null);
    try {
      const result = await api.detectSmallFolders(settings, threshold);
      setFolders(result);
      setUnassigned(result);
      setMergeGroups([]);
      setDetected(true);
    } catch (e) {
      setMessage({ text: `検出エラー: ${e}`, type: "error" });
    }
    setLoading(false);
  };

  const addGroup = () => {
    setMergeGroups((prev) => [
      ...prev,
      { name: `まとめ${prev.length + 1}`, items: [] },
    ]);
  };

  const assignToGroup = (folder: SmallFolder, groupIndex: number) => {
    setUnassigned((prev) => prev.filter((f) => f.name !== folder.name));
    setMergeGroups((prev) =>
      prev.map((g, i) =>
        i === groupIndex ? { ...g, items: [...g.items, folder] } : g,
      ),
    );
  };

  const unassignFromGroup = (folder: SmallFolder, groupIndex: number) => {
    setMergeGroups((prev) =>
      prev.map((g, i) =>
        i === groupIndex
          ? { ...g, items: g.items.filter((f) => f.name !== folder.name) }
          : g,
      ),
    );
    setUnassigned((prev) => [...prev, folder]);
  };

  const updateGroupName = (index: number, name: string) => {
    setMergeGroups((prev) =>
      prev.map((g, i) => (i === index ? { ...g, name } : g)),
    );
  };

  const removeGroup = (index: number) => {
    const group = mergeGroups[index];
    setUnassigned((prev) => [...prev, ...group.items]);
    setMergeGroups((prev) => prev.filter((_, i) => i !== index));
  };

  const handleMerge = async () => {
    const groups: MergeGroup[] = mergeGroups
      .filter((g) => g.items.length > 0)
      .map((g) => ({
        target_name: g.name,
        source_names: g.items.map((f) => f.name),
      }));

    if (groups.length === 0) {
      setMessage({ text: "統合対象がありません", type: "warning" });
      return;
    }

    try {
      const warnings = await api.mergeSmallFolders(settings, groups);
      if (warnings.length > 0) {
        setMessage({ text: `統合完了（警告: ${warnings.join(", ")}）`, type: "warning" });
      } else {
        setMessage({ text: "統合が完了しました", type: "success" });
        closeTimerRef.current = setTimeout(onClose, 1000);
      }
    } catch (e) {
      setMessage({ text: `統合エラー: ${e}`, type: "error" });
    }
  };

  return (
    <div className="p-4">
      {message && (
        <InlineMessage message={message.text} type={message.type} onDismiss={() => setMessage(null)} />
      )}

      <div className="flex items-center gap-2 mb-4">
        <label className="text-sm">しきい値:</label>
        <input
          type="number"
          min={1}
          className="border rounded px-2 py-1 text-sm w-16"
          value={threshold}
          onChange={(e) => setThreshold(Number(e.target.value))}
        />
        <span className="text-sm">件以下</span>
        <button
          className="px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
          onClick={detect}
          disabled={loading}
        >
          {loading ? "検出中..." : "検出"}
        </button>
      </div>

      {detected && folders.length === 0 && (
        <p className="text-center text-gray-500 py-4 text-sm">
          対象フォルダは見つかりませんでした
        </p>
      )}

      {detected && folders.length > 0 && (
        <>
          <p className="text-xs text-gray-500 mb-2">
            検出結果: {folders.length}件
          </p>

          <div className="space-y-3 mb-3">
            {mergeGroups.map((group, gi) => (
              <div key={`group-${group.name}`} className="border rounded p-2">
                <div className="flex items-center gap-2 mb-1">
                  <span className="text-xs text-gray-500">
                    グループ{gi + 1}:
                  </span>
                  <input
                    type="text"
                    className="border rounded px-1 py-0.5 text-sm flex-1"
                    value={group.name}
                    onChange={(e) => updateGroupName(gi, e.target.value)}
                  />
                  <button
                    className="text-xs text-red-500 hover:text-red-700"
                    onClick={() => removeGroup(gi)}
                  >
                    削除
                  </button>
                </div>
                {group.items.map((folder) => (
                  <div
                    key={folder.name}
                    className="flex items-center gap-2 pl-4 text-sm"
                  >
                    <span>
                      {folder.name} ({folder.file_count}件)
                    </span>
                    <button
                      className="text-xs text-gray-400 hover:text-red-600"
                      onClick={() => unassignFromGroup(folder, gi)}
                    >
                      ×
                    </button>
                  </div>
                ))}
              </div>
            ))}
          </div>

          <button
            className="text-xs text-blue-600 hover:underline mb-3"
            onClick={addGroup}
          >
            + グループ追加
          </button>

          {unassigned.length > 0 && (
            <div className="border-t pt-2">
              <p className="text-xs text-gray-500 mb-1">未割り当て</p>
              <div className="space-y-0.5">
                {unassigned.map((folder) => (
                  <div
                    key={folder.name}
                    className="flex items-center gap-2 text-sm"
                  >
                    <span>
                      {folder.name} ({folder.file_count}件)
                    </span>
                    {mergeGroups.length > 0 && (
                      <select
                        className="text-xs border rounded px-1"
                        value=""
                        onChange={(e) => {
                          if (e.target.value)
                            assignToGroup(folder, Number(e.target.value));
                        }}
                      >
                        <option value="">割り当て先...</option>
                        {mergeGroups.map((g) => (
                          <option key={`opt-${g.name}`} value={mergeGroups.indexOf(g)}>
                            {g.name}
                          </option>
                        ))}
                      </select>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          <div className="flex justify-end gap-2 border-t pt-3 mt-3">
            <button
              className="px-4 py-1.5 text-sm border rounded hover:bg-gray-100"
              onClick={onClose}
            >
              キャンセル
            </button>
            <button
              className="px-4 py-1.5 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
              onClick={handleMerge}
            >
              実行
            </button>
          </div>
        </>
      )}
    </div>
  );
}
