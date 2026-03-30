import { useRef, useMemo, useState, useCallback } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import type { DryRunResult } from "../lib/types";

interface Props {
  data: DryRunResult[];
  editedDestinations: Map<string, string>;
  onDestinationChange: (originalDest: string, newDest: string) => void;
}

const ROW_HEIGHT = 28;

export default function PreviewTable({
  data,
  editedDestinations,
  onDestinationChange,
}: Props) {
  const parentRef = useRef<HTMLDivElement>(null);
  const [editingFilePath, setEditingFilePath] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");

  // 編集済みdestinationを適用したデータ
  const displayData = useMemo(() => {
    return data.map((row) => {
      if (row.destination && editedDestinations.has(row.destination)) {
        return {
          ...row,
          destination: editedDestinations.get(row.destination)!,
        };
      }
      return row;
    });
  }, [data, editedDestinations]);

  // 分類先フォルダでグループ化ソート
  const sortedData = useMemo(() => {
    return [...displayData].sort((a, b) => {
      const statusOrder = { classifiable: 0, unclassifiable: 1 };
      const sa = statusOrder[a.status];
      const sb = statusOrder[b.status];
      if (sa !== sb) return sa - sb;
      const da = a.destination ?? "";
      const db = b.destination ?? "";
      if (da !== db) return da.localeCompare(db, "ja");
      return a.file_name.localeCompare(b.file_name, "ja");
    });
  }, [displayData]);

  const virtualizer = useVirtualizer({
    count: sortedData.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 20,
  });

  const startEdit = useCallback(
    (row: DryRunResult) => {
      if (row.status !== "classifiable" || !row.destination) return;
      setEditingFilePath(row.file_path);
      setEditValue(row.destination);
    },
    [],
  );

  const commitEdit = useCallback(
    (originalRow: DryRunResult) => {
      const trimmed = editValue.trim();
      // 元のdestination（編集前のオリジナル）を取得
      const origFromData = data.find((r) => r.file_path === originalRow.file_path);
      const originalDest = origFromData?.destination ?? "";

      if (trimmed && trimmed !== originalRow.destination) {
        onDestinationChange(originalDest, trimmed);
      }
      setEditingFilePath(null);
    },
    [editValue, data, onDestinationChange],
  );

  const cancelEdit = useCallback(() => {
    setEditingFilePath(null);
  }, []);

  return (
    <div ref={parentRef} className="h-full overflow-y-auto overflow-x-hidden">
      {/* ヘッダー */}
      <div className="sticky top-0 bg-gray-50 z-10 flex border-b text-xs font-medium text-gray-700">
        <div className="flex-[5] min-w-0 px-2 py-1.5">ファイル名</div>
        <div className="flex-[3] min-w-0 px-2 py-1.5">分類先</div>
        <div className="w-16 shrink-0 px-2 py-1.5 text-center">状態</div>
      </div>

      {/* 仮想スクロール領域 */}
      <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
        {virtualizer.getVirtualItems().map((virtualRow) => {
          const row = sortedData[virtualRow.index];
          const isEditing = editingFilePath === row.file_path;
          // 元データと比較して編集済みかどうかを判定
          const origRow = data.find((r) => r.file_path === row.file_path);
          const isEdited =
            origRow?.destination !== null &&
            editedDestinations.has(origRow?.destination ?? "");

          return (
            <div
              key={row.file_path}
              className={`flex items-center border-b border-gray-100 text-xs ${
                isEdited ? "bg-yellow-50" : "hover:bg-gray-50"
              }`}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                height: ROW_HEIGHT,
                transform: `translateY(${virtualRow.start}px)`,
              }}
            >
              <div className="flex-[5] min-w-0 px-2 truncate">
                {row.file_name}
              </div>
              <div className="flex-[3] min-w-0 px-2 truncate">
                {isEditing ? (
                  <input
                    className="w-full border rounded px-1 py-0 text-xs bg-white"
                    value={editValue}
                    onChange={(e) => setEditValue(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") commitEdit(row);
                      if (e.key === "Escape") cancelEdit();
                    }}
                    onBlur={() => commitEdit(row)}
                    autoFocus
                  />
                ) : row.status === "classifiable" ? (
                  <span
                    className="cursor-pointer hover:text-blue-700 block truncate"
                    onClick={() => startEdit(row)}
                    title="クリックで編集"
                  >
                    {row.destination ?? "—"}
                  </span>
                ) : (
                  <span>{row.destination ?? "—"}</span>
                )}
              </div>
              <div className="w-16 shrink-0 px-2 text-center">
                {row.status === "classifiable" && (
                  <span className="text-green-700">○</span>
                )}
                {row.status === "unclassifiable" && (
                  <span className="text-gray-400">分類不可</span>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
