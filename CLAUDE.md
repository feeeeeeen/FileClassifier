# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Language

すべての回答・コメント・コミットメッセージは日本語で行うこと。

## Project Overview

ファイル分類デスクトップGUIツール。ファイル名の `[タグ名]` パターンに基づき、辞書を参照してフォルダへ自動分類する。日本語の作者名・サークル名での分類が主な用途。

- **EXE名**: `FileClassifier.exe`
- **対象OS**: Windows 10 (21H2+) / Windows 11
- **要件定義書**: `requirements_definition.md`（日本語）が仕様の原典

## Tech Stack

| Layer | Technology |
|---|---|
| Framework | Tauri v2 |
| Backend | Rust |
| Frontend | React + TypeScript |
| UI | Tailwind CSS |
| 仮想スクロール | TanStack Virtual |
| Data | JSON files (settings.json + folder_dictionary.json) |

## Build & Dev Commands

```bash
# EXEビルド + release/へコピー（動作確認はすべてこのEXEで行う）
npm run tauri:build
# 出力先: release/FileClassifier.exe

# Rustテスト
cd src-tauri && cargo test

# TypeScript型チェック
npx tsc --noEmit

# フロントエンドビルド（Viteのみ）
npm run build
```

## 動作確認

動作確認はすべて `release/FileClassifier.exe` で行う。
`cargo tauri dev` や `npm run dev` による開発サーバーでの確認は行わない。

## Architecture

**フロント/バックエンド分離が厳密**: ファイル操作・辞書管理・類似判定・正規化などのコアロジックはすべてRust側。フロントエンドはUI表示と操作のみ。

- **Tauri Commands** (`invoke`): フロントエンド→バックエンドRPC
- **Tauri Events**: バックエンド→フロントエンド非同期通知（進捗等）

### Rustモジュール構成 (src-tauri/src/)

| モジュール | 責務 |
|---|---|
| `normalize.rs` | 正規化（NFKC→小文字→空白除去→文字フィルタ）+ 類似判定用拡張正規化 |
| `tag.rs` | タグ抽出 `[...]`、パストラバーサル対策、ファイル名補正 |
| `dictionary.rs` | 辞書の読み書き・フォーマットチェック・Main(Sub)グループ化ルール |
| `classifier.rs` | ファイルスキャン、ドライラン、分類実行、衝突回避、中断対応 |
| `similarity.rs` | 類似フォルダ検出(BFSクラスタリング)、類似辞書マッチング、統合処理 |
| `settings.rs` | 設定ファイル読み書き、アプリケーションルート管理 |
| `commands.rs` | Tauriコマンド定義（フロントエンドとのインターフェース）+ AppState管理 |
| `types.rs` | 共通型定義 |

### 分類フロー（重要）

ファイル分類時の分類先解決は以下の順序で行う:

0. **プレビュー編集による上書き**: ユーザーがプレビュー上で分類先を編集した場合、その値を最優先で使用
1. **辞書完全一致**: `normalize(タグ)` が辞書キーに一致 → そのフォルダ
2. **類似マッチ**: `find_similar_dict_match` で辞書の全フォルダ名をMain(Sub)解析し、3.4Aと同じ4条件（メインキー一致/サブキー完全一致/キーセット一致/包摂関係）+ 辞書キー同士の類似比較 → 一致フォルダ（分類実行時に辞書登録）
3. **自動作成**: タグ名をそのままフォルダ名として使用（辞書に自動登録）

タグ抽出できないファイルのみ「分類不可」(Unclassifiable)。それ以外は必ず分類先が決まる。

各ステップのマッチ種別は`MatchType`（DictExact/DictSimilar/AutoCreated）として`DryRunResult`に含まれ、フロントエンドに返される。

### プレビュー分類先編集

プレビューテーブルのdestinationセルはクリックでインライン編集可能。編集はoriginal_destination単位でまとめて反映される（同じ分類先の全ファイルが一括変更）。

分類実行時に`DestinationOverride`としてバックエンドに送信され、分類前に辞書更新が行われる:
- **AutoCreated**: タグ→新名のマッピングを辞書に追加 + 新名のMain(Sub)キー生成
- **DictExact/DictSimilar**: 旧名に紐づく全キーの値を新名に更新 + 旧名・新名からキー生成

### UI構成

- **メインウィンドウ**: 上部（入出力フォルダ設定）+ 下部（プレビューテーブル/仮想スクロール/分類先インライン編集）+ 右サイドバー（アクション/オプション）
- **右ペイン**: 分類実行ボタン、分類オプション（トグルスイッチ4つ）、辞書編集、フォルダ整理、ログ表示。全ボタンにホバーツールチップあり
- **辞書編集画面**: テーブルレイアウト（左列=フォルダ名、右列=キーバッジ）。辞書作成もこの画面から実行。HOME/ENDキーでスクロール対応
- **出力フォルダ整理**: モーダルダイアログ。類似統合/少数フォルダ整理の2モード
- **ログ表示画面**: フィルタリング付きログ一覧

### 分類オプション

| オプション | デフォルト | 説明 |
|---|---|---|
| 再帰スキャン | OFF | サブフォルダも再帰的にスキャン。入出力フォルダ同一時は強制無効（グレーアウト） |
| コピー作成 | OFF (=移動) | ONでコピー、OFFで移動 |
| 著者名削除 | OFF | ファイル名から `[タグ]` 部分を削除（連続空白は圧縮） |
| 数字半角統一 | OFF | 全角数字→半角+前後空白除去 |

## Important Design Decisions

- UIテキスト・エラーメッセージはすべて**日本語**
- アプリルート = EXEのディレクトリ（開発時はプロジェクトルート）。settings.jsonと辞書ファイルはここに配置
- **設定は自動保存**: フォルダパス・オプション変更時に即座にsettings.jsonへ保存、次回起動時に復元
- 辞書はプレーンなJSONファイルでユーザーが外部エディタで直接編集可能。読み込み時にフォーマットチェック(F-08)
- タグ抽出時は**パストラバーサル対策**必須（`/`, `\`, `:`, `..`, 先頭`.`を除去）
- ファイル操作は非同期（UIブロックなし）、進捗はTauriイベントで通知
- ドライランは自動再実行: アプリ起動時、入力フォルダ変更時、辞書更新時、スキャンオプション変更時
- **入出力フォルダ同一を許可**: 再帰スキャンは強制無効化される（バックエンド・フロントエンド両方で制御）。フォルダ直下のファイルをサブフォルダに分類するユースケースを想定
- 辞書編集画面では辞書変更後に`loadSettings`を呼ばないこと（メモリ上の変更がディスクの辞書で上書きされるため、`loadDictionary`のみで再取得する）
