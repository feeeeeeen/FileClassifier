import { useState, useEffect } from "react";
import * as api from "../lib/api";
import type { LogEntry } from "../lib/types";

interface Props {
  onBack: () => void;
}

type Filter = "all" | "success" | "skip" | "error";

export default function LogViewer({ onBack }: Props) {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [filter, setFilter] = useState<Filter>("all");
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    api.getLogs().then(setLogs).catch((e) => {
      setLoadError(`ログの読み込みに失敗しました: ${e}`);
    });
  }, []);

  const filteredLogs = logs.filter((log) => {
    if (filter === "all") return true;
    if (filter === "success")
      return log.action === "copy" || log.action === "move";
    if (filter === "skip") return log.action === "skip";
    if (filter === "error") return log.action === "error";
    return true;
  });

  const handleClear = async () => {
    try {
      await api.clearLogs();
      setLogs([]);
    } catch (e) {
      console.error("ログクリアエラー:", e);
    }
  };

  return (
    <div className="flex flex-col h-screen">
      <div className="border-b p-3 flex items-center gap-3 shrink-0">
        <button
          className="px-3 py-1 text-sm border rounded hover:bg-gray-100"
          onClick={onBack}
        >
          ← 戻る
        </button>
        <h2 className="text-sm font-bold">処理ログ</h2>
        <div className="flex-1" />

        <div className="flex gap-1 text-xs">
          {(
            [
              ["all", "すべて"],
              ["success", "成功"],
              ["skip", "スキップ"],
              ["error", "エラー"],
            ] as const
          ).map(([value, label]) => (
            <button
              key={value}
              className={`px-2 py-1 rounded ${
                filter === value
                  ? "bg-blue-600 text-white"
                  : "border hover:bg-gray-100"
              }`}
              onClick={() => setFilter(value)}
            >
              {label}
            </button>
          ))}
        </div>

        <button
          className="px-3 py-1 text-sm border rounded hover:bg-gray-100 text-red-600"
          onClick={handleClear}
        >
          クリア
        </button>
      </div>

      <div className="flex-1 overflow-auto">
        {loadError ? (
          <div className="bg-red-50 border border-red-200 rounded m-4 px-4 py-3 text-sm text-red-700">
            {loadError}
          </div>
        ) : filteredLogs.length === 0 ? (
          <p className="text-sm text-gray-500 text-center py-8">
            ログがありません
          </p>
        ) : (
          <table className="w-full text-sm">
            <thead className="sticky top-0 bg-gray-50">
              <tr>
                <th className="text-left px-3 py-1.5 border-b font-medium">
                  操作
                </th>
                <th className="text-left px-3 py-1.5 border-b font-medium">
                  アクション
                </th>
                <th className="text-left px-3 py-1.5 border-b font-medium">
                  ソース
                </th>
                <th className="text-left px-3 py-1.5 border-b font-medium">
                  分類先
                </th>
                <th className="text-left px-3 py-1.5 border-b font-medium">
                  詳細
                </th>
              </tr>
            </thead>
            <tbody>
              {filteredLogs.map((log) => (
                <tr key={`${log.timestamp}-${log.operation}-${log.source}-${log.destination}`} className="hover:bg-gray-50">
                  <td className="px-3 py-1 border-b border-gray-100">
                    {log.operation}
                  </td>
                  <td className="px-3 py-1 border-b border-gray-100">
                    <span
                      className={
                        log.action === "error"
                          ? "text-red-600"
                          : log.action === "skip"
                            ? "text-gray-500"
                            : "text-green-700"
                      }
                    >
                      {log.action}
                    </span>
                  </td>
                  <td
                    className="px-3 py-1 border-b border-gray-100 truncate max-w-xs"
                    title={log.source}
                  >
                    {log.source}
                  </td>
                  <td
                    className="px-3 py-1 border-b border-gray-100 truncate max-w-xs"
                    title={log.destination}
                  >
                    {log.destination}
                  </td>
                  <td className="px-3 py-1 border-b border-gray-100 text-gray-500">
                    {log.detail}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
