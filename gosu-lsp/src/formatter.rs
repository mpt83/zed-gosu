#[derive(Clone, Copy)]
pub struct FormattingOptions {
    pub tab_size: usize,
    pub insert_spaces: bool,
}

impl Default for FormattingOptions {
    fn default() -> Self {
        Self {
            tab_size: 2,
            insert_spaces: true,
        }
    }
}

pub fn format_gosu(source: &str, options: FormattingOptions) -> String {
    let indent_unit = if options.insert_spaces {
        " ".repeat(options.tab_size)
    } else {
        "\t".to_string()
    };

    let mut lines = Vec::new();
    let mut indent_level = 0_usize;
    let mut in_block_comment = false;
    let mut previous_blank = false;

    for line in source.lines() {
        let trimmed_end = line.trim_end_matches([' ', '\t']);
        let trimmed = trimmed_end.trim_start_matches([' ', '\t']);

        if trimmed.is_empty() {
            if previous_blank {
                continue;
            }
            lines.push(String::new());
            previous_blank = true;
            continue;
        }
        previous_blank = false;

        let normalized = normalize_spacing(trimmed);

        let starts_in_block_comment = in_block_comment;
        let leading_closes = if starts_in_block_comment {
            0
        } else {
            leading_closing_braces(&normalized)
        };
        let line_indent = indent_level.saturating_sub(leading_closes);
        let (opens, closes) = count_braces(&normalized, &mut in_block_comment);

        lines.push(format!("{}{}", indent_unit.repeat(line_indent), normalized));

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

fn normalize_spacing(line: &str) -> String {
    if should_normalize_type_colons(line) {
        normalize_type_colons(line)
    } else {
        line.to_string()
    }
}

fn should_normalize_type_colons(line: &str) -> bool {
    line.contains("function ")
        || line.starts_with("var ")
        || line.contains(" var ")
        || line.starts_with("property get ")
        || line.starts_with("property set ")
}

fn normalize_type_colons(line: &str) -> String {
    let limit = line.find('=').unwrap_or(line.len());
    let mut output = String::with_capacity(line.len());
    let mut chars = line.char_indices().peekable();
    let mut quote = None;
    let mut escaped = false;

    while let Some((index, ch)) = chars.next() {
        if index >= limit {
            output.push(ch);
            output.extend(chars.map(|(_, ch)| ch));
            break;
        }

        if let Some(active_quote) = quote {
            output.push(ch);

            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active_quote {
                quote = None;
            }

            continue;
        }

        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                output.push(ch);
            }
            '/' if chars.peek().is_some_and(|(_, next)| *next == '/') => {
                output.push(ch);
                output.extend(chars.map(|(_, ch)| ch));
                break;
            }
            ':' => {
                while output.ends_with([' ', '\t']) {
                    output.pop();
                }

                output.push(':');

                while chars.peek().is_some_and(|(_, next)| matches!(next, ' ' | '\t')) {
                    chars.next();
                }

                if chars.peek().is_some_and(|(_, next)| !matches!(next, ')' | ',' | '{')) {
                    output.push(' ');
                }
            }
            _ => output.push(ch),
        }
    }

    output
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

    #[test]
    fn normalizes_type_colons_and_collapses_blank_lines() {
        let source = "package example.formatting\n\n\nuses example.JsonParser\nuses example.Logger\n\npublic class SampleMessage {\npublic var primaryValue: String\npublic var secondaryValue : String\n\n\npublic static function fromText(text : String) : SampleMessage {\nreturn new JsonParser().parse(text, SampleMessage)\n}\n\nfunction writeLog(logger : Logger) {\nlogger.info(\"Sample message\")\nlogger.info(\"  [primary: \" + this.primaryValue + \"]\")\nlogger.info(\"  [secondary: \" + this.secondaryValue + \"]\")\n}\n}\n";
        let expected = "package example.formatting\n\nuses example.JsonParser\nuses example.Logger\n\npublic class SampleMessage {\n  public var primaryValue: String\n  public var secondaryValue: String\n\n  public static function fromText(text: String): SampleMessage {\n    return new JsonParser().parse(text, SampleMessage)\n  }\n\n  function writeLog(logger: Logger) {\n    logger.info(\"Sample message\")\n    logger.info(\"  [primary: \" + this.primaryValue + \"]\")\n    logger.info(\"  [secondary: \" + this.secondaryValue + \"]\")\n  }\n}\n";

        assert_eq!(format_gosu(source, default_options()), expected);
    }
}
