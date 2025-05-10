# code_context

コードベースからコンテキスト情報を作成するシンプルなツール

- treeコマンド相当の情報
- 各ファイルの相対パス + ファイルの内容(デフォルトは先頭200行)

## 使い方

```bash
code_context [OPTIONS] <DIRECTORY_PATH>
```

### 利用可能なオプション

```txt
<DIRECTORY_PATH>    分析するルートディレクトリパス

-q, --query <QUERY>    プロンプトに含める質問（指定しない場合、コンテキスト情報のみが出力されます）
-s, --system-prompt <PROMPT>    システムプロンプトテンプレート（クエリが指定された場合のみ表示）[デフォルト値あり]
-n, --head-lines <LINES>    各ファイルの先頭から表示する行数（0ですべての行）[デフォルト: 200]
--filter-tree    ディレクトリツリー表示にフィルターを適用
--include-ext <EXT>    含めるファイル拡張子（例: --include-ext 'py' --include-ext 'js'）
--exclude-ext <EXT>    除外するファイル拡張子
--include-path <REGEX>    含めるパスパターン（正規表現）
--exclude-path <REGEX>    除外するパスパターン（正規表現）
--include-gitignore    .gitignoreファイルを無視する
-h, --help    ヘルプ情報を表示
-V, --version    バージョン情報を表示
```

### 実行例

```bash
# srcディレクトリのコードコンテキストを取得
code_context src

# 特定の質問と共にコンテキストを生成
code_context -q "このコードの主な機能は何ですか？" src

# 拡張子でフィルタリング（Rustファイルのみ）
code_context --include-ext rs src

# 特定のパスパターンを除外
code_context --exclude-path "test|spec" src

# 全行を表示
code_context -n 0 src
```

#### 出力例

```txt
**以下はコンテキスト情報**

ルートパス: /path/to/project/src
ファイル構造:
 .
 └─ main.rs

フィルタリングオプション:
 - .gitignoreファイルで指定されたファイルを無視する:いいえ
 - ツリー表示にもフィルタを適用する:いいえ

各ファイルの先頭200行:

```main.rs
use clap::Parser;
// ... (ファイルの内容が表示されます)
```

## インストール

### ダウンロード

環境に合った実行ファイルを[リリースページ](https://github.com/o02c/code-context/releases)からダウンロードできます。

ダウンロード後、ファイル名の変更が必要な場合があります（例: `code_context-macos-arm64` → `code_context`）。

### ソースからのビルド

```bash
# リポジトリをクローン
git clone https://github.com/o02c/code-context.git
cd code-context

# ビルド
cargo build --release

# 実行ファイルは target/release/ に生成されます
```
