use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut server = Server {
        reader: BufReader::new(stdin.lock()),
        writer: stdout.lock(),
        documents: HashMap::new(),
    };

    server.run()
}

struct Server<R, W> {
    reader: BufReader<R>,
    writer: W,
    documents: HashMap<String, String>,
}

impl<R: Read, W: Write> Server<R, W> {
    fn run(&mut self) -> io::Result<()> {
        while let Some(message) = self.read_message()? {
            self.handle_message(message)?;
        }

        Ok(())
    }

    fn handle_message(&mut self, message: Value) -> io::Result<()> {
        let method = message.get("method").and_then(Value::as_str);
        let id = message.get("id").cloned();

        match method {
            Some("initialize") => {
                self.respond(
                    id,
                    json!({
                        "capabilities": {
                            "documentFormattingProvider": true,
                            "textDocumentSync": {
                                "openClose": true,
                                "change": 1
                            }
                        },
                        "serverInfo": {
                            "name": "gosu-lsp",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }),
                )?;
            }
            Some("shutdown") => {
                self.respond(id, Value::Null)?;
            }
            Some("textDocument/didOpen") => {
                if let Some((uri, text)) = did_open_text(&message) {
                    self.documents.insert(uri, text);
                }
            }
            Some("textDocument/didChange") => {
                if let Some((uri, text)) = did_change_text(&message) {
                    self.documents.insert(uri, text);
                }
            }
            Some("textDocument/didClose") => {
                if let Some(uri) = text_document_uri(&message) {
                    self.documents.remove(&uri);
                }
            }
            Some("textDocument/formatting") => {
                let edits = self.formatting_edits(&message);
                self.respond(id, Value::Array(edits))?;
            }
            Some("initialized") | Some("exit") => {}
            Some(_) => {
                if id.is_some() {
                    self.error(id, -32601, "method not found")?;
                }
            }
            None => {}
        }

        Ok(())
    }

    fn formatting_edits(&self, message: &Value) -> Vec<Value> {
        let Some(uri) = text_document_uri(message) else {
            return Vec::new();
        };

        let Some(text) = self.documents.get(&uri) else {
            return Vec::new();
        };

        let formatted = format_gosu(text, formatting_options(message));
        if formatted == *text {
            return Vec::new();
        }

        vec![json!({
            "range": whole_document_range(text),
            "newText": formatted
        })]
    }

    fn read_message(&mut self) -> io::Result<Option<Value>> {
        let mut content_length = None;
        let mut line = String::new();

        loop {
            line.clear();
            let bytes = self.reader.read_line(&mut line)?;
            if bytes == 0 {
                return Ok(None);
            }

            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                break;
            }

            if let Some(value) = trimmed.strip_prefix("Content-Length:") {
                content_length = value.trim().parse::<usize>().ok();
            }
        }

        let Some(content_length) = content_length else {
            return Ok(None);
        };

        let mut body = vec![0; content_length];
        self.reader.read_exact(&mut body)?;
        let message = serde_json::from_slice(&body).unwrap_or(Value::Null);
        Ok(Some(message))
    }

    fn respond(&mut self, id: Option<Value>, result: Value) -> io::Result<()> {
        let Some(id) = id else {
            return Ok(());
        };

        self.write_message(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        }))
    }

    fn error(&mut self, id: Option<Value>, code: i64, message: &str) -> io::Result<()> {
        let Some(id) = id else {
            return Ok(());
        };

        self.write_message(json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": code,
                "message": message
            }
        }))
    }

    fn write_message(&mut self, message: Value) -> io::Result<()> {
        let body = serde_json::to_vec(&message).expect("serializing LSP response");
        write!(self.writer, "Content-Length: {}\r\n\r\n", body.len())?;
        self.writer.write_all(&body)?;
        self.writer.flush()
    }
}

fn did_open_text(message: &Value) -> Option<(String, String)> {
    let text_document = message.get("params")?.get("textDocument")?;
    Some((
        text_document.get("uri")?.as_str()?.to_string(),
        text_document.get("text")?.as_str()?.to_string(),
    ))
}

fn did_change_text(message: &Value) -> Option<(String, String)> {
    let params = message.get("params")?;
    let uri = params
        .get("textDocument")?
        .get("uri")?
        .as_str()?
        .to_string();
    let text = params
        .get("contentChanges")?
        .as_array()?
        .last()?
        .get("text")?
        .as_str()?
        .to_string();

    Some((uri, text))
}

fn text_document_uri(message: &Value) -> Option<String> {
    message
        .get("params")?
        .get("textDocument")?
        .get("uri")?
        .as_str()
        .map(ToString::to_string)
}

#[derive(Clone, Copy)]
struct FormattingOptions {
    tab_size: usize,
    insert_spaces: bool,
}

impl Default for FormattingOptions {
    fn default() -> Self {
        Self {
            tab_size: 2,
            insert_spaces: true,
        }
    }
}

fn formatting_options(message: &Value) -> FormattingOptions {
    let Some(options) = message.get("params").and_then(|params| params.get("options")) else {
        return FormattingOptions::default();
    };

    FormattingOptions {
        tab_size: options
            .get("tabSize")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value > 0)
            .unwrap_or(2),
        insert_spaces: options
            .get("insertSpaces")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    }
}

fn whole_document_range(text: &str) -> Value {
    let mut line = 0_u32;
    let mut character = 0_u32;

    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += ch.len_utf16() as u32;
        }
    }

    json!({
        "start": { "line": 0, "character": 0 },
        "end": { "line": line, "character": character }
    })
}

fn format_gosu(source: &str, options: FormattingOptions) -> String {
    let indent_unit = if options.insert_spaces {
        " ".repeat(options.tab_size)
    } else {
        "\t".to_string()
    };

    let mut lines = Vec::new();
    let mut indent_level = 0_usize;
    let mut in_block_comment = false;

    for line in source.lines() {
        let trimmed_end = line.trim_end_matches([' ', '\t']);
        let trimmed = trimmed_end.trim_start_matches([' ', '\t']);

        if trimmed.is_empty() {
            lines.push(String::new());
            continue;
        }

        let starts_in_block_comment = in_block_comment;
        let leading_closes = if starts_in_block_comment {
            0
        } else {
            leading_closing_braces(trimmed)
        };
        let line_indent = indent_level.saturating_sub(leading_closes);
        let (opens, closes) = count_braces(trimmed, &mut in_block_comment);

        lines.push(format!("{}{}", indent_unit.repeat(line_indent), trimmed));

        indent_level = apply_brace_delta(indent_level, opens, closes);
    }

    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn leading_closing_braces(line: &str) -> usize {
    line.chars().take_while(|ch| *ch == '}').count()
}

fn apply_brace_delta(indent_level: usize, opens: usize, closes: usize) -> usize {
    indent_level.saturating_add(opens).saturating_sub(closes)
}

fn count_braces(line: &str, in_block_comment: &mut bool) -> (usize, usize) {
    let mut opens = 0;
    let mut closes = 0;
    let mut chars = line.chars().peekable();
    let mut quote = None;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if *in_block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                *in_block_comment = false;
            }
            continue;
        }

        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
                continue;
            }

            if ch == '\\' {
                escaped = true;
                continue;
            }

            if ch == active_quote {
                quote = None;
            }

            continue;
        }

        match ch {
            '"' | '\'' => quote = Some(ch),
            '/' if chars.peek() == Some(&'/') => break,
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                *in_block_comment = true;
            }
            '{' => opens += 1,
            '}' => closes += 1,
            _ => {}
        }
    }

    (opens, closes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_options() -> FormattingOptions {
        FormattingOptions::default()
    }

    #[test]
    fn trims_trailing_whitespace_and_final_blank_lines() {
        assert_eq!(
            format_gosu("class A {  \n}\t\n\n", default_options()),
            "class A {\n}\n"
        );
    }

    #[test]
    fn preserves_empty_file() {
        assert_eq!(format_gosu("", default_options()), "");
    }

    #[test]
    fn indents_nested_brace_blocks_with_spaces() {
        let source = "class Foo {\nfunction bar() {\nif (x) {\nreturn\n}\n}\n}\n";
        let expected = "class Foo {\n  function bar() {\n    if (x) {\n      return\n    }\n  }\n}\n";

        assert_eq!(format_gosu(source, default_options()), expected);
    }

    #[test]
    fn indents_nested_brace_blocks_with_tabs() {
        let options = FormattingOptions {
            insert_spaces: false,
            ..FormattingOptions::default()
        };
        let source = "class Foo {\nfunction bar() {\nreturn\n}\n}\n";
        let expected = "class Foo {\n\tfunction bar() {\n\t\treturn\n\t}\n}\n";

        assert_eq!(format_gosu(source, options), expected);
    }

    #[test]
    fn keeps_else_at_closing_brace_indent() {
        let source = "if (x) {\nreturn\n} else {\nreturn\n}\n";
        let expected = "if (x) {\n  return\n} else {\n  return\n}\n";

        assert_eq!(format_gosu(source, default_options()), expected);
    }

    #[test]
    fn ignores_braces_in_strings_and_comments() {
        let source =
            "class Foo {\nvar text = \"}\" // {\n/* {\n} */\nfunction bar() {\nreturn\n}\n}\n";
        let expected = "class Foo {\n  var text = \"}\" // {\n  /* {\n  } */\n  function bar() {\n    return\n  }\n}\n";

        assert_eq!(format_gosu(source, default_options()), expected);
    }
}
