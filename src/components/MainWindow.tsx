import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import * as api from "../lib/api";
import type {
  Settings,
  DryRunResult,
  ClassifyProgress,
  ClassifySummary,
  DestinationOverride,
} from "../lib/types";
import { defaultSettings } from "../lib/types";
import PreviewTable from "./PreviewTable";
import FolderCleanupDialog from "./FolderCleanupDialog";

interface Props {
  onOpenDictionary: () => void;
  onOpenLog: () => void;
}

export default function MainWindow({ onOpenDictionary, onOpenLog }: Props) {
  const [settings, setSettings] = useState<Settings>(defaultSettings);
  const [preview, setPreview] = useState<DryRunResult[]>([]);
  const [isScanning, setIsScanning] = useState(false);
  const [isClassifying, setIsClassifying] = useState(false);
  const [progress, setProgress] = useState<ClassifyProgress | null>(null);
  const [statusMessage, setStatusMessage] = useState("待機中");
  const [showCleanup, setShowCleanup] = useState(false);
  // 分類先の編集: originalDest → newDest
  const [editedDestinations, setEditedDestinations] = useState<Map<string, string>>(new Map());
  const dryRunTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // 初期化: 設定読み込み
  useEffect(() => {
    api.loadSettings().then((s) => {
      setSettings(s);
    }).catch((e) => {
      setStatusMessage(`設定読み込みエラー: ${e}`);
    });
  }, []);

  // 進捗イベントリスナー
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;
    listen<ClassifyProgress>("classify-progress", (event) => {
      setProgress(event.payload);
    }).then((fn) => { unlistenFn = fn; });
    return () => { unlistenFn?.(); };
  }, []);

  // settingsの最新値をrefで保持（useEffect内から参照するため）
  const settingsRef = useRef(settings);
  settingsRef.current = settings;

  // ドライラン実行（デバウンス付き）
  const triggerDryRun = useCallback(() => {
    if (dryRunTimer.current) clearTimeout(dryRunTimer.current);
    dryRunTimer.current = setTimeout(async () => {
      const s = settingsRef.current;
      if (!s.input_dir) return;
      setIsScanning(true);
      try {
        const results = await api.runDryRun(s);
        setPreview(results);
        setEditedDestinations(new Map());
      } catch {
        setPreview([]);
      }
      setIsScanning(false);
    }, 300);
  }, []);

  // 設定変更時にドライラン自動再実行
  useEffect(() => {
    triggerDryRun();
  }, [settings.input_dir, settings.output_dir, settings.recursive_scan, triggerDryRun]);

  // タイマーのクリーンアップ
  useEffect(() => {
    return () => {
      if (dryRunTimer.current) clearTimeout(dryRunTimer.current);
    };
  }, []);

  const updateSettings = (partial: Partial<Settings>) => {
    setSettings((prev) => {
      const updated = { ...prev, ...partial };
      api.saveSettings(updated).catch((e) => {
        setStatusMessage(`設定保存エラー: ${e}`);
      });
      return updated;
    });
  };

  const updateOptions = (
    partial: Partial<Settings["options"]>,
  ) => {
    setSettings((prev) => {
      const updated = { ...prev, options: { ...prev.options, ...partial } };
      api.saveSettings(updated).catch((e) => {
        setStatusMessage(`設定保存エラー: ${e}`);
      });
      return updated;
    });
  };

  const selectFolder = async (field: "input_dir" | "output_dir") => {
    const selected = await open({ directory: true });
    if (selected) {
      updateSettings({ [field]: selected });
    }
  };

  const handleClassify = async () => {
    setIsClassifying(true);
    setStatusMessage("分類中...");
    setProgress(null);
    try {
      await api.saveSettings(settings);

      // editedDestinations → DestinationOverride[] を構築
      const overrides: DestinationOverride[] = [];
      for (const [originalDest, newDest] of editedDestinations) {
        // この originalDest を持つ全ファイルの情報を収集
        const affectedRows = preview.filter(
          (r) => r.destination === originalDest && r.tag && r.match_type,
        );
        for (const row of affectedRows) {
          overrides.push({
            file_path: row.file_path,
            original_destination: originalDest,
            new_destination: newDest,
            tag: row.tag!,
            match_type: row.match_type!,
          });
        }
      }

      const summary: ClassifySummary = await api.runClassify(settings, overrides);
      setStatusMessage(
        `完了 — 成功: ${summary.success}件, スキップ: ${summary.skipped}件, エラー: ${summary.errors}件`,
      );
      triggerDryRun();
    } catch (e) {
      setStatusMessage(`エラー: ${e}`);
    }
    setIsClassifying(false);
    setProgress(null);
  };

  const handleCancel = async () => {
    try {
      await api.cancelClassify();
      setStatusMessage("中断しました");
    } catch (e) {
      setStatusMessage(`中断操作エラー: ${e}`);
    }
  };

  // 入出力フォルダ同一判定
  const isSameDir =
    settings.input_dir !== "" &&
    settings.output_dir !== "" &&
    settings.input_dir.replace(/[\\/]+$/, "").toLowerCase() ===
      settings.output_dir.replace(/[\\/]+$/, "").toLowerCase();

  // サマリ計算
  const classifiable = preview.filter(
    (r) => r.status === "classifiable",
  ).length;
  const unclassifiable = preview.filter(
    (r) => r.status === "unclassifiable",
  ).length;

  return (
    <div className="flex h-screen">
      {/* メインエリア */}
      <div className="flex flex-1 flex-col min-w-0">
        {/* 上部ペイン: 設定 */}
        <div className="border-b p-4 space-y-3 shrink-0">
          <div className="flex items-center gap-2">
            <label className="w-24 text-sm font-medium shrink-0">
              入力フォルダ:
            </label>
            <input
              type="text"
              className="flex-1 border rounded px-2 py-1 text-sm bg-white"
              value={settings.input_dir}
              onChange={(e) => updateSettings({ input_dir: e.target.value })}
            />
            <button
              className="px-3 py-1 text-sm border rounded hover:bg-gray-100"
              onClick={() => selectFolder("input_dir")}
            >
              参照
            </button>
          </div>
          <div className="flex items-center gap-2">
            <label className="w-24 text-sm font-medium shrink-0">
              出力フォルダ:
            </label>
            <input
              type="text"
              className="flex-1 border rounded px-2 py-1 text-sm bg-white"
              value={settings.output_dir}
              onChange={(e) => updateSettings({ output_dir: e.target.value })}
            />
            <button
              className="px-3 py-1 text-sm border rounded hover:bg-gray-100"
              onClick={() => selectFolder("output_dir")}
            >
              参照
            </button>
          </div>
        </div>

        {/* 下部ペイン: プレビュー */}
        <div className="flex-1 flex flex-col min-h-0">
          <div className="px-4 py-2 text-sm text-gray-600 border-b flex gap-4 shrink-0">
            {isScanning ? (
              <span>スキャン中...</span>
            ) : (
              <>
                <span>
                  分類可能:{" "}
                  <strong className="text-green-700">{classifiable}</strong>
                </span>
                <span>
                  分類不可:{" "}
                  <strong className="text-gray-500">{unclassifiable}</strong>
                </span>
              </>
            )}
          </div>

          <div className="flex-1 min-h-0">
            <PreviewTable
              data={preview}
              editedDestinations={editedDestinations}
              onDestinationChange={(originalDest, newDest) => {
                setEditedDestinations((prev) => {
                  const next = new Map(prev);
                  if (originalDest === newDest) {
                    next.delete(originalDest);
                  } else {
                    next.set(originalDest, newDest);
                  }
                  return next;
                });
              }}
            />
          </div>

          {/* 進捗バー */}
          <div className="border-t px-4 py-2 text-sm flex items-center gap-3 shrink-0">
            <span className="text-gray-600">{statusMessage}</span>
            {isClassifying && progress && (
              <>
                <div className="flex-1 bg-gray-200 rounded-full h-2">
                  <div
                    className="bg-blue-600 h-2 rounded-full transition-all"
                    style={{
                      width: `${progress.total > 0 ? (progress.current / progress.total) * 100 : 0}%`,
                    }}
                  />
                </div>
                <span className="text-xs text-gray-500">
                  {progress.current}/{progress.total}
                </span>
                <button
                  className="px-2 py-0.5 text-xs border rounded hover:bg-red-50 text-red-600 border-red-300"
                  onClick={handleCancel}
                >
                  停止
                </button>
              </>
            )}
          </div>
        </div>
      </div>

      {/* 右ペイン: アクション */}
      <div className="w-44 border-l p-3 flex flex-col gap-2 shrink-0">
        <ActionButton
          primary
          onClick={handleClassify}
          disabled={isClassifying || !settings.input_dir || !settings.output_dir}
          label="分類実行"
          tooltip="プレビューの内容に従い、ファイルを分類先フォルダへ振り分けます"
        />

        <hr className="my-1" />
        <span className="text-xs text-gray-500 font-medium">分類オプション</span>
        <ToggleButton
          active={settings.recursive_scan && !isSameDir}
          onClick={() => updateSettings({ recursive_scan: !settings.recursive_scan })}
          label="再帰スキャン"
          tooltip={isSameDir
            ? "入出力フォルダが同一のため再帰スキャンは無効です"
            : "入力フォルダ配下のサブフォルダも再帰的にスキャンします"}
          disabled={isSameDir}
        />
        <ToggleButton
          active={!settings.is_move_mode}
          onClick={() => updateSettings({ is_move_mode: !settings.is_move_mode })}
          label="コピー作成"
          tooltip="ONでファイルをコピー（元ファイルを残す）、OFFで移動します"
        />
        <ToggleButton
          active={settings.options.remove_tag}
          onClick={() => updateOptions({ remove_tag: !settings.options.remove_tag })}
          label="著者名削除"
          tooltip="分類先へのコピー/移動時にファイル名から [著者名] 部分を削除します"
        />
        <ToggleButton
          active={settings.options.normalize_numbers}
          onClick={() => updateOptions({ normalize_numbers: !settings.options.normalize_numbers })}
          label="数字半角統一"
          tooltip="全角数字を半角に変換し、数字前後の空白を除去します"
        />

        <hr className="my-1" />
        <ActionButton
          onClick={onOpenDictionary}
          label="辞書編集"
          tooltip="辞書の分類キーとフォルダ名を編集・作成します"
        />

        <hr className="my-1" />
        <ActionButton
          onClick={() => setShowCleanup(true)}
          disabled={!settings.output_dir}
          label="フォルダ整理"
          tooltip="類似フォルダの統合や、少数ファイルフォルダの整理を行います"
        />

        <hr className="my-1" />
        <ActionButton
          onClick={onOpenLog}
          label="ログ表示"
          tooltip="分類・統合などの処理結果ログを確認します"
        />
      </div>

      {/* 出力フォルダ整理ダイアログ */}
      {showCleanup && (
        <FolderCleanupDialog
          settings={settings}
          onClose={() => {
            setShowCleanup(false);
            triggerDryRun();
          }}
        />
      )}
    </div>
  );
}

function ToggleButton({
  active,
  onClick,
  label,
  tooltip,
  disabled,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
  tooltip: string;
  disabled?: boolean;
}) {
  return (
    <button
      className={`relative group w-full flex items-center gap-2 py-1 px-2 rounded transition-colors ${
        disabled ? "opacity-40 cursor-not-allowed" : "hover:bg-gray-50"
      }`}
      onClick={disabled ? undefined : onClick}
      disabled={disabled}
    >
      {/* スイッチ */}
      <span
        className={`relative inline-block w-7 h-4 rounded-full shrink-0 transition-colors ${
          active ? "bg-blue-600" : "bg-gray-300"
        }`}
      >
        <span
          className={`absolute top-0.5 left-0.5 w-3 h-3 bg-white rounded-full shadow transition-transform ${
            active ? "translate-x-3" : ""
          }`}
        />
      </span>
      <span className="text-xs text-gray-700 text-left leading-tight">{label}</span>
      {/* ツールチップ: 右ペインの左側に表示 */}
      <Tooltip text={tooltip} />
    </button>
  );
}

function ActionButton({
  onClick,
  disabled,
  label,
  tooltip,
  primary,
}: {
  onClick: () => void;
  disabled?: boolean;
  label: string;
  tooltip: string;
  primary?: boolean;
}) {
  return (
    <button
      className={`relative group w-full text-sm rounded disabled:opacity-50 ${
        primary
          ? "py-2 font-medium bg-blue-600 text-white hover:bg-blue-700"
          : "py-1.5 border hover:bg-gray-100"
      }`}
      onClick={onClick}
      disabled={disabled}
    >
      {label}
      <Tooltip text={tooltip} />
    </button>
  );
}

function Tooltip({ text }: { text: string }) {
  return (
    <div className="pointer-events-none absolute right-full top-1/2 -translate-y-1/2 mr-2 z-50 hidden group-hover:block w-52 px-2.5 py-1.5 text-xs text-left text-white bg-gray-800 rounded shadow-lg whitespace-normal leading-relaxed font-normal">
      {text}
    </div>
  );
}
