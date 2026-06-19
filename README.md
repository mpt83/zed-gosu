# Zed Gosu Extension

Gosu support for Zed.

Includes:

- Tree-sitter syntax highlighting
- A Rust-backed `gosu-lsp`
- Brace-aware document formatting

## Local Install

1. Open Zed.
2. Run `zed: install dev extension`.
3. Select this `zed-gosu` directory.
4. Open a `.gs`, `.gsx`, `.gsp`, or `.gst` file.

For local LSP testing before a release:

```sh
cd gosu-lsp
cargo install --path .
```

## Format On Save

```json
{
  "languages": {
    "Gosu": {
      "format_on_save": "on"
    }
  }
}
```

Formatting currently:

- indents nested brace blocks
- trims trailing spaces and tabs
- removes extra blank lines at EOF
- ensures one final newline
