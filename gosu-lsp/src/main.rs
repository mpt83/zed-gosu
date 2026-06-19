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

        let formatted = format_gosu(text);
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

fn format_gosu(source: &str) -> String {
    let mut lines = source
        .lines()
        .map(|line| line.trim_end_matches([' ', '\t']).to_string())
        .collect::<Vec<_>>();

    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_trailing_whitespace_and_final_blank_lines() {
        assert_eq!(format_gosu("class A {  \n}\t\n\n"), "class A {\n}\n");
    }

    #[test]
    fn preserves_empty_file() {
        assert_eq!(format_gosu(""), "");
    }
}
