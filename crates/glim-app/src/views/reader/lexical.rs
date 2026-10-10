//! Small, viewport-only lexical highlighter for formats without bundled grammars.
//! Keeps a cheap Rope snapshot, never a full-file token or decoration array.
use gpui_kit::{
    component::input::{
        EditorState, FoldRange, HighlightStyleResolver, InputEdit, InputHighlighter, Rope, RopeExt,
    },
    *,
};
use std::ops::Range;
pub(super) fn supports(language: &str) -> bool {
    matches!(
        language,
        "dockerfile" | "ini" | "dotenv" | "ignore" | "jinja" | "nix"
    )
}
pub(super) struct Lexical {
    language: SharedString,
    text: Rope,
}
impl Lexical {
    pub fn new(language: &str) -> Self {
        Self {
            language: language.to_owned().into(),
            text: Rope::new(),
        }
    }
}
impl InputHighlighter for Lexical {
    fn language(&self) -> SharedString {
        self.language.clone()
    }
    fn update(
        &mut self,
        _: Option<InputEdit>,
        text: &Rope,
        _: bool,
        _: &mut Window,
        _: &mut Context<EditorState>,
    ) {
        self.text = text.clone();
    }
    fn fold_ranges(&self, _: &Rope) -> Vec<FoldRange> {
        Vec::new()
    }
    fn styles(
        &self,
        range: &Range<usize>,
        resolver: &dyn HighlightStyleResolver,
    ) -> Vec<(Range<usize>, HighlightStyle)> {
        if range.is_empty() {
            return Vec::new();
        }
        let row = self.text.offset_to_point(range.start).row;
        let start = self.text.line_start_offset(row);
        let end = self
            .text
            .line_end_offset(self.text.offset_to_point(range.end).row);
        let text = self.text.slice(start..end).to_string();
        let mut runs = Vec::new();
        let mut offset = start;
        for line in text.split_inclusive('\n') {
            for (span, kind) in tokens(line, &self.language) {
                let span =
                    (offset + span.start).max(range.start)..(offset + span.end).min(range.end);
                if !span.is_empty() {
                    runs.push((span, resolver.style(kind).unwrap_or_default()));
                }
            }
            offset += line.len();
        }
        runs
    }
}
fn tokens(line: &str, language: &str) -> Vec<(Range<usize>, &'static str)> {
    let bytes = line.as_bytes();
    let mut result = Vec::new();
    let mut i = 0;
    let first = line.len() - line.trim_start().len();
    let key_end = line.find('=');
    while i < bytes.len() {
        let start = i;
        let mut kind = "";
        if (i == first && (bytes[i] == b'#' || (language == "ini" && bytes[i] == b';')))
            || (language == "jinja" && line[i..].starts_with("{#"))
        {
            kind = "comment";
            i = if language == "jinja" {
                line[i..].find("#}").map_or(bytes.len(), |n| i + n + 2)
            } else {
                bytes.len()
            };
        } else if bytes[i] == b'\'' || bytes[i] == b'"' {
            kind = "string";
            let quote = bytes[i];
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' {
                    i = (i + 2).min(bytes.len());
                } else if bytes[i] == quote {
                    i += 1;
                    break;
                } else {
                    i += 1;
                }
            }
        } else if language == "ini" && i == first && bytes[i] == b'[' {
            kind = "type";
            i = line[i..].find(']').map_or(bytes.len(), |n| i + n + 1);
        } else if language == "jinja"
            && ["{{", "}}", "{%", "%}", "</", "/>", "<!"]
                .iter()
                .any(|s| line[i..].starts_with(s))
        {
            kind = "punctuation.bracket";
            i += 2;
        } else if bytes[i] == b'$' {
            kind = "variable";
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || b"_{}".contains(&bytes[i]))
            {
                i += 1;
            }
        } else if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric() || b"_-.".contains(&bytes[i]))
            {
                i += 1;
            }
            let word = &line[start..i];
            if (language == "dockerfile" && start == first)
                || matches!(language, "jinja" | "nix")
                    && matches!(
                        word,
                        "if" | "else"
                            | "elif"
                            | "endif"
                            | "for"
                            | "in"
                            | "endfor"
                            | "set"
                            | "block"
                            | "endblock"
                            | "extends"
                            | "include"
                            | "macro"
                            | "endmacro"
                            | "import"
                            | "from"
                            | "as"
                            | "filter"
                            | "endfilter"
                            | "with"
                            | "endwith"
                            | "not"
                            | "and"
                            | "or"
                            | "let"
                            | "inherit"
                            | "rec"
                            | "assert"
                            | "then"
                            | "builtins"
                    )
            {
                kind = "keyword";
            } else if matches!(
                word,
                "true" | "false" | "null" | "none" | "True" | "False" | "None"
            ) {
                kind = "constant";
            } else if matches!(language, "ini" | "dotenv") && key_end.is_some_and(|end| start < end)
            {
                kind = "property";
            } else if language == "jinja" && start > 0 && bytes[start - 1] == b'<' {
                kind = "tag";
            }
        } else if bytes[i].is_ascii_digit() {
            kind = "number";
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
        } else {
            if b"={}[]():,|<>".contains(&bytes[i]) {
                kind = "punctuation.delimiter";
            }
            if language == "ignore" && b"!*?/".contains(&bytes[i]) {
                kind = "keyword";
            }
            i += line[i..].chars().next().unwrap().len_utf8();
        }
        // Quoted strings may have arbitrary UTF-8; advancing bytewise there ends on ASCII delimiters.
        result.push((start..i, kind));
    }
    result
}
