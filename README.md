# prompt_context_builder

A tool for extracting context information from a codebase and generating prompts

## Usage

```bash
prompt_context_builder [OPTIONS] <DIRECTORY_PATH>
```

### Available Options

```
<DIRECTORY_PATH>    Root directory path to analyze

-q, --query <QUERY>    Question to include in the prompt (if not specified, only context information is output)
-s, --system-prompt <PROMPT>    System prompt template (only displayed when a query is specified) [default value provided]
-n, --head-lines <LINES>    Number of lines to display from the beginning of each file (0 for all lines) [default: 200]
--filter-tree    Apply filters to the directory tree display
--include-ext <EXT>    File extensions to include (e.g., --include-ext 'py' --include-ext 'js')
--exclude-ext <EXT>    File extensions to exclude
--include-path <REGEX>    Path patterns to include (regex)
--exclude-path <REGEX>    Path patterns to exclude (regex)
--include-gitignore    Don't respect .gitignore files
-h, --help    Display help information
-V, --version    Display version information
```

## Installation

### Download

You can download the executable for your environment from the [releases page](https://github.com/o02c/project-prompt-gen/releases).

After downloading, you may need to rename the file (e.g., `prompt_context_builder-macos-arm64` → `prompt_context_builder`).

### Build from Source

```bash
# Clone the repository
git clone https://github.com/o02c/project-prompt-gen.git
cd project-prompt-gen

# Build
cargo build --release

# The executable will be generated in target/release/
```
