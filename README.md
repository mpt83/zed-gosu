# Zed Gosu Extension

Gosu support for Zed.

Includes:

- Tree-sitter syntax highlighting
- A Rust-backed `gosu-lsp`
- Whitespace-only document formatting

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

- trims trailing spaces and tabs
- removes extra blank lines at EOF
- ensures one final newline

## Publishing

The extension points to:

```text
https://github.com/mpt83/zed-gosu
https://github.com/mpt83/tree-sitter-gosu
```

For one-click installs, publish `gosu-lsp` release assets from this repo:

```sh
./scripts/package-gosu-lsp.sh aarch64-apple-darwin
./scripts/package-gosu-lsp.sh x86_64-apple-darwin
./scripts/package-gosu-lsp.sh x86_64-unknown-linux-gnu
./scripts/package-gosu-lsp.sh aarch64-unknown-linux-gnu
```

Expected asset names:

```text
gosu-lsp-aarch64-apple-darwin.tar.gz
gosu-lsp-x86_64-apple-darwin.tar.gz
gosu-lsp-x86_64-unknown-linux-gnu.tar.gz
gosu-lsp-aarch64-unknown-linux-gnu.tar.gz
```

The grammar repo must include generated Tree-sitter sources, including
`src/parser.c`, at the `rev` pinned in `extension.toml`.
