use clap::Parser;
use ignore::WalkBuilder;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::collections::{HashSet, BTreeMap};
use std::error::Error;
use anyhow::{Context, Result};
// ★修正: Node は Tree enum の一部なので、直接インポートしない
use ascii_tree::{Tree, /* Node, */ write_tree};
use tera::{Tera, Context as TeraContext};
use serde::Serialize;


/// CLI arguments definition
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    /// Root directory path to analyze
    #[arg()]
    root_path: PathBuf,
    
    /// Optional query to include in the prompt
    #[arg(long, short='q')]
    query: Option<String>,
    
    /// System prompt template to use (only displayed when a query is specified)
    #[arg(long, short='s', default_value = "熟練のITエンジニアとして、コンテキスト情報のみから以下の質問に回答してください。追加で必要な情報があれば適宜質問してください。")]
    system_prompt: String,
    
    /// Number of lines to display from the beginning of each file (0 for all lines)
    #[arg(long, short='n', default_value_t = 200)]
    head_lines: usize,
    
    /// Apply filters to the directory tree display
    #[arg(long, action = clap::ArgAction::SetTrue)]
    filter_tree: bool,
    
    /// File extensions to include (e.g., 'py', 'js')
    #[arg(long, value_name = "EXT", action = clap::ArgAction::Append)]
    include_ext: Vec<String>,
    
    /// File extensions to exclude
    #[arg(long, value_name = "EXT", action = clap::ArgAction::Append)]
    exclude_ext: Vec<String>,
    
    /// Path patterns to include (regex)
    #[arg(long, value_name = "REGEX", action = clap::ArgAction::Append)]
    include_path: Vec<String>,
    
    /// Path patterns to exclude (regex)
    #[arg(long, value_name = "REGEX", action = clap::ArgAction::Append)]
    exclude_path: Vec<String>,
    
    /// Don't respect .gitignore files
    #[arg(long, action = clap::ArgAction::SetTrue)]
    include_gitignore: bool,
}

/// Teraテンプレート用オプション情報
#[derive(Serialize)]
struct TemplateOptions<'a> {
    include_ext: &'a Vec<String>,
    exclude_ext: &'a Vec<String>,
    include_path: &'a Vec<String>,
    exclude_path: &'a Vec<String>,
    include_gitignore: bool,
    filter_tree: bool,
}

/// Tree構築用一時ノード (変更なし)
#[derive(Default, Debug)]
struct TempNode {
    children: BTreeMap<String, TempNode>,
}

// ★修正: 戻り値の型を自作 Node から ascii_tree::Tree に変更
// また、戻り値を Tree::Node バリアントとして構築する
fn convert_to_ascii_node(name: String, temp_node: &TempNode) -> Tree {
    // 子ノードも再帰的に Tree 型として取得
    let children: Vec<Tree> = temp_node.children.iter()
        .map(|(child_name, child_node)| {
            convert_to_ascii_node(child_name.clone(), child_node)
        })
        .collect();
    // ascii_tree の Tree::Node バリアントを返す
    Tree::Node(name, children)
}

// ★修正: 戻り値の型を Option<Node> から Option<Tree> に変更
// また、戻り値を Tree::Node バリアントとして構築する
fn build_ascii_tree_nodes(files: &[PathBuf], root_path: &Path) -> Option<Tree> {
     if files.is_empty() { return None; }

     let mut root = TempNode::default();

     for path in files {
         let relative_path = path.strip_prefix(root_path).unwrap_or(path);
         let mut current_node = &mut root;
         let components: Vec<_> = relative_path.components()
                                             .map(|c| c.as_os_str().to_string_lossy().into_owned())
                                             .collect();
         for component in components {
             current_node = current_node.children.entry(component).or_default();
         }
     }

     // TempNode を ascii_tree::Tree に変換
     let mut top_level_children: Vec<Tree> = Vec::new(); // 型を Vec<Tree> に変更
     for (name, node) in root.children.iter() {
         top_level_children.push(convert_to_ascii_node(name.clone(), node)); // 修正された関数を呼ぶ
     }

     // ルートノード "." を持つ Tree::Node を返す
     Some(Tree::Node(".".to_string(), top_level_children))
}

/// Teraテンプレート文字列 (変更なし)
const OUTPUT_TEMPLATE: &str = r#"
{% if query %}
{{ system_prompt }}

---

質問: {{ query }}

---
{% endif %}

**以下はコンテキスト情報**

ルートパス: {{ root_path | safe }}
ファイル構造:
{{ tree_context | safe }}

フィルタリングオプション:
{%- if options.include_ext | length > 0 %}
 - 含める拡張子: {{ options.include_ext | json_encode() | safe }}
{%- endif -%}
{%- if options.exclude_ext | length > 0 %}
 - 除外する拡張子: {{ options.exclude_ext | json_encode() | safe }}
{%- endif -%}
{%- if options.include_path | length > 0 %}
 - 含めるパス (Regex): {{ options.include_path | json_encode() | safe }}
{%- endif -%}
{%- if options.exclude_path | length > 0 %}
 - 除外するパス (Regex): {{ options.exclude_path | json_encode() | safe }}
{%- endif %}
 - .gitignoreファイルで指定されたファイルを無視する: {%- if options.include_gitignore %}はい{% else %}いいえ{%- endif %}
 - ツリー表示にもフィルタを適用する: {%- if options.filter_tree %}はい{% else %}いいえ{%- endif %}

各ファイルの{% if file_head_lines == 0 %}全内容{% else %}先頭{{ file_head_lines }}行{% endif %}:
{%- if file_contents and file_contents | length > 0 %}
{{ file_contents | safe }}
{%- else %}
(該当するファイルが見つからないか、内容を読み込めませんでした)
{%- endif %}

"#;


fn main() -> Result<()> {
    let args = CliArgs::parse();

    let root_path = args.root_path.canonicalize()
        .with_context(|| format!("ルートパスが見つからないか、アクセスできません: {:?}", args.root_path))?;

    // フィルタ条件準備 (変更なし)
    let include_exts: HashSet<String> = args.include_ext.iter().map(|s| s.to_lowercase()).collect();
    let exclude_exts: HashSet<String> = args.exclude_ext.iter().map(|s| s.to_lowercase()).collect();
    let include_path_regs: Result<Vec<Regex>, _> = args.include_path.iter()
        .map(|p| Regex::new(p).with_context(|| format!("無効な正規表現（include-path）: {}", p)))
        .collect();
    let include_path_regs = include_path_regs?;
     let exclude_path_regs: Result<Vec<Regex>, _> = args.exclude_path.iter()
        .map(|p| Regex::new(p).with_context(|| format!("無効な正規表現（exclude-path）: {}", p)))
        .collect();
    let exclude_path_regs = exclude_path_regs?;

    // ツリー表示用のファイル探索
    // デフォルトでは拡張子やパスのフィルタを適用せず、gitignoreのみ適用する
    let mut tree_files_set = HashSet::new();
    
    // .gitignoreファイル自体を追加
    let gitignore_path = root_path.join(".gitignore");
    if gitignore_path.exists() && gitignore_path.is_file() {
        tree_files_set.insert(gitignore_path.clone());
    }
    
    // ツリー表示用のファイル探索
    let tree_walker = WalkBuilder::new(&root_path)
        .hidden(false)
        .git_ignore(!args.include_gitignore)
        .build();
    for result in tree_walker {
        let entry = match result { Ok(entry) => entry, Err(_err) => { /* 省略 */ continue; } };
        let path = entry.path();
        if path.components().any(|comp| comp.as_os_str() == ".git") { continue; }
        if !entry.file_type().map_or(false, |ft| ft.is_file()) { continue; }
        
        // filter_treeが指定されている場合のみ、拡張子やパスのフィルタを適用する
        if args.filter_tree {
            let relative_path = match path.strip_prefix(&root_path) { Ok(p) => p, Err(_) => path, };
            // 拡張子フィルタ
            if let Some(ext) = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) { 
                if !include_exts.is_empty() && !include_exts.contains(&ext) { continue; } 
                if exclude_exts.contains(&ext) { continue; } 
            } else if !include_exts.is_empty() { continue; }
            // パスフィルタ
            let path_str_for_regex = relative_path.to_string_lossy();
            if exclude_path_regs.iter().any(|reg| reg.is_match(&path_str_for_regex)) { continue; }
            if !include_path_regs.is_empty() && !include_path_regs.iter().any(|reg| reg.is_match(&path_str_for_regex)) { continue; }
        }
        
        tree_files_set.insert(entry.path().to_path_buf());
    }
    
    // コンテンツ表示用のファイル探索とフィルタリング
    // 重複を防ぐためにHashSetを使用
    let mut matched_files_set = HashSet::new();
    
    // .gitignoreファイル自体を追加
    if gitignore_path.exists() && gitignore_path.is_file() {
        matched_files_set.insert(gitignore_path.clone());
    }
    
    let walker = WalkBuilder::new(&root_path)
        .hidden(false)
        .git_ignore(!args.include_gitignore)
        .build();
    for result in walker {
        let entry = match result { Ok(entry) => entry, Err(_err) => { /* 省略 */ continue; } };
        let path = entry.path();
        if path.components().any(|comp| comp.as_os_str() == ".git") { continue; }
        if !entry.file_type().map_or(false, |ft| ft.is_file()) { continue; }
        let relative_path = match path.strip_prefix(&root_path) { Ok(p) => p, Err(_) => path, };
        // 拡張子フィルタ (省略)
        if let Some(ext) = path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()) { if !include_exts.is_empty() && !include_exts.contains(&ext) { continue; } if exclude_exts.contains(&ext) { continue; } } else if !include_exts.is_empty() { continue; }
        // パスフィルタ (省略)
        let path_str_for_regex = relative_path.to_string_lossy();
        if exclude_path_regs.iter().any(|reg| reg.is_match(&path_str_for_regex)) { continue; }
        if !include_path_regs.is_empty() && !include_path_regs.iter().any(|reg| reg.is_match(&path_str_for_regex)) { continue; }
        matched_files_set.insert(entry.path().to_path_buf());
    }

    // --- コンテキストの生成 ---
    // HashSetからVecに変換してソート
    let mut matched_files: Vec<PathBuf> = matched_files_set.into_iter().collect();
    matched_files.sort();
    
    // ツリー表示用のファイルをソート
    let mut tree_files: Vec<PathBuf> = tree_files_set.into_iter().collect();
    tree_files.sort();

    // ★修正: build_ascii_tree_nodes は Option<Tree> を返す
    let tree_node_opt: Option<Tree> = build_ascii_tree_nodes(&tree_files, &root_path);
    let tree_context = match tree_node_opt {
        // ★修正: Some から取り出すのは Tree 型の値
        Some(tree_root) => {
            let mut buffer = String::new();
            // ★修正: write_tree に Tree 型の値を渡す
            write_tree(&mut buffer, &tree_root)
                .map(|_| buffer)
                .unwrap_or_else(|e| format!("(ツリー表示エラー: {})", e))
        },
        None => "(該当するファイルが見つかりませんでした)".to_string(),
    };


    // ファイル内容コンテキストの生成
    let mut file_contents = String::new();
    let file_head_lines: usize = args.head_lines;
    for path in &matched_files {
        let relative_path = path.strip_prefix(&root_path).unwrap_or(path);
        file_contents.push_str(&format!("\n```{}\n", relative_path.display()));
        match read_head_lines(path, file_head_lines) {
            Ok(content) => file_contents.push_str(&content),
            Err(e) => file_contents.push_str(&format!("(読み込みエラー: {})\n", e)),
        }
        file_contents.push_str("```\n");
    }
    file_contents = file_contents.trim_end().to_string();


    // --- Teraテンプレートを使って最終出力を生成 ---
    let template_options = TemplateOptions {
        include_ext: &args.include_ext,
        exclude_ext: &args.exclude_ext,
        include_path: &args.include_path,
        exclude_path: &args.exclude_path,
        include_gitignore: args.include_gitignore,
        filter_tree: args.filter_tree,
    };
    let mut context = TeraContext::new();
    // root_pathはHTMLエスケープされないようにsafeフィルターを使用するため、
    // ここでは単純な文字列として挿入
    context.insert("root_path", &root_path.display().to_string());
    context.insert("options", &template_options);
    if let Some(query) = &args.query {
        context.insert("query", query);
    }
    context.insert("system_prompt", &args.system_prompt);
    context.insert("tree_context", &tree_context);
    context.insert("file_contents", &file_contents);
    context.insert("file_head_lines", &file_head_lines);

    // Teraテンプレートのレンダリング
    // エラーが発生した場合は詳細なエラー情報を表示
    let final_output = match Tera::one_off(OUTPUT_TEMPLATE, &context, false) {
        Ok(output) => output,
        Err(e) => {
            eprintln!("\n\nテンプレートエラー: {}", e);
            if let Some(source) = e.source() {
                eprintln!("\n原因: {}", source);
            }
            return Err(anyhow::anyhow!("テンプレートのレンダリングに失敗しました"));
        }
    };

    // --- 最終結果を出力 --- (変更なし)
    print!("{}", final_output);

    Ok(())
}

/// ファイルの先頭 N 行を読み込むヘルパー関数
/// n_linesが0の場合は全量読み込み
fn read_head_lines(path: &Path, n_lines: usize) -> Result<String> {
    let file = File::open(path)
        .with_context(|| format!("ファイルを開けませんでした: {:?}", path))?;
    let reader = BufReader::new(file);
    let mut result = String::new();
    
    // n_linesが0の場合は全量読み込み
    if n_lines == 0 {
        for line_result in reader.lines() {
            let line = line_result
                .with_context(|| format!("ファイル内容の読み込みに失敗しました: {:?}", path))?;
            result.push_str(&line);
            result.push('\n');
        }
    } else {
        // 指定行数のみ読み込み
        for (i, line_result) in reader.lines().enumerate() {
            if i >= n_lines {
                break;
            }
            let line = line_result
                .with_context(|| format!("ファイル内容の読み込みに失敗しました: {:?}", path))?;
            result.push_str(&line);
            result.push('\n');
        }
    }
    
    Ok(result)
}