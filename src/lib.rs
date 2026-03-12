use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    message: String,
    line: usize,
    column: usize,
}

impl ParseError {
    fn new(input: &str, offset: usize, message: impl Into<String>) -> Self {
        let mut line = 1;
        let mut column = 1;

        for ch in input[..offset.min(input.len())].chars() {
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }

        Self {
            message: message.into(),
            line,
            column,
        }
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at line {}, column {}",
            self.message, self.line, self.column
        )
    }
}

impl Error for ParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FormatOptions {
    pub comma_style: CommaStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommaStyle {
    #[default]
    None,
    Commas,
    Trailing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Document {
    root: Root,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Root {
    Object { entries: Vec<Entry>, braced: bool },
    Array(Vec<Value>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    Field(Field),
    Include(Include),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    path: Vec<String>,
    op: FieldOp,
    value: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldOp {
    Set,
    Append,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Include {
    Bare(StringLiteral),
    File(StringLiteral),
    Url(StringLiteral),
    Classpath(StringLiteral),
    Required(Box<Include>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    Single(ValuePart),
    Concat(Vec<ConcatItem>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConcatItem {
    separator: String,
    part: ValuePart,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ValuePart {
    Object(Vec<Entry>),
    Array(Vec<Value>),
    Atom(Atom),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Atom {
    String(StringLiteral),
    Number(String),
    Boolean(String),
    Null,
    Unquoted(String),
    Substitution(Substitution),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StringLiteral {
    raw: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Substitution {
    optional: bool,
    path: Vec<String>,
    list_hint: bool,
}

pub fn format_hocon(input: &str) -> Result<String, ParseError> {
    format_hocon_with_options(input, FormatOptions::default())
}

pub fn format_hocon_with_options(
    input: &str,
    options: FormatOptions,
) -> Result<String, ParseError> {
    let mut parser = Parser::new(input);
    let document = parser.parse_document()?;
    Ok(document.format(options))
}

impl Document {
    fn format(&self, options: FormatOptions) -> String {
        match &self.root {
            Root::Object { entries, braced } => {
                if *braced {
                    format!("{}\n", format_object(entries, 0, options))
                } else if entries.is_empty() {
                    String::new()
                } else {
                    format_root_entries(entries, options)
                }
            }
            Root::Array(items) => format!("{}\n", format_array(items, 0, options)),
        }
    }
}

fn format_root_entries(entries: &[Entry], options: FormatOptions) -> String {
    let mut out = String::new();
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&format_entry(entry, 0, options));
        if should_add_comma(index, entries.len(), options.comma_style) {
            out.push(',');
        }
    }
    out.push('\n');
    out
}

fn format_object(entries: &[Entry], indent: usize, options: FormatOptions) -> String {
    if entries.is_empty() {
        return "{}".to_string();
    }

    let mut out = String::new();
    out.push_str("{\n");
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&" ".repeat(indent + 2));
        out.push_str(&format_entry(entry, indent + 2, options));
        if should_add_comma(index, entries.len(), options.comma_style) {
            out.push(',');
        }
    }
    out.push('\n');
    out.push_str(&" ".repeat(indent));
    out.push('}');
    out
}

fn format_array(items: &[Value], indent: usize, options: FormatOptions) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }

    let mut out = String::new();
    out.push_str("[\n");
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str(&" ".repeat(indent + 2));
        out.push_str(&format_value(item, indent + 2, options));
        if should_add_comma(index, items.len(), options.comma_style) {
            out.push(',');
        }
    }
    out.push('\n');
    out.push_str(&" ".repeat(indent));
    out.push(']');
    out
}

fn should_add_comma(index: usize, len: usize, comma_style: CommaStyle) -> bool {
    match comma_style {
        CommaStyle::None => false,
        CommaStyle::Commas => index + 1 < len,
        CommaStyle::Trailing => true,
    }
}

fn format_entry(entry: &Entry, indent: usize, options: FormatOptions) -> String {
    match entry {
        Entry::Field(field) => {
            let operator = match field.op {
                FieldOp::Set => "=",
                FieldOp::Append => "+=",
            };
            format!(
                "{} {} {}",
                format_path(&field.path, true),
                operator,
                format_value(&field.value, indent, options)
            )
        }
        Entry::Include(include) => format_include(include),
    }
}

fn format_include(include: &Include) -> String {
    match include {
        Include::Bare(path) => format!("include {}", path.raw),
        Include::File(path) => format!("include file({})", path.raw),
        Include::Url(path) => format!("include url({})", path.raw),
        Include::Classpath(path) => format!("include classpath({})", path.raw),
        Include::Required(inner) => match inner.as_ref() {
            Include::Bare(path) => format!("include required({})", path.raw),
            Include::File(path) => format!("include required(file({}))", path.raw),
            Include::Url(path) => format!("include required(url({}))", path.raw),
            Include::Classpath(path) => format!("include required(classpath({}))", path.raw),
            Include::Required(_) => unreachable!("nested required() is normalized during parsing"),
        },
    }
}

fn format_value(value: &Value, indent: usize, options: FormatOptions) -> String {
    match value {
        Value::Single(part) => format_value_part(part, indent, options),
        Value::Concat(items) => {
            let mut out = String::new();
            for item in items {
                out.push_str(&item.separator);
                out.push_str(&format_value_part(&item.part, indent, options));
            }
            out
        }
    }
}

fn format_value_part(part: &ValuePart, indent: usize, options: FormatOptions) -> String {
    match part {
        ValuePart::Object(entries) => format_object(entries, indent, options),
        ValuePart::Array(items) => format_array(items, indent, options),
        ValuePart::Atom(atom) => format_atom(atom),
    }
}

fn format_atom(atom: &Atom) -> String {
    match atom {
        Atom::String(literal) => literal.raw.clone(),
        Atom::Number(number) | Atom::Boolean(number) | Atom::Unquoted(number) => number.clone(),
        Atom::Null => "null".to_string(),
        Atom::Substitution(substitution) => {
            let optional = if substitution.optional { "?" } else { "" };
            let list_hint = if substitution.list_hint { "[]" } else { "" };
            format!(
                "${{{}{}{}}}",
                optional,
                format_path(&substitution.path, false),
                list_hint
            )
        }
    }
}

fn format_path(path: &[String], quote_leading_include: bool) -> String {
    path.iter()
        .enumerate()
        .map(|(index, segment)| format_path_segment(segment, quote_leading_include && index == 0))
        .collect::<Vec<_>>()
        .join(".")
}

fn format_path_segment(segment: &str, quote_include: bool) -> String {
    if quote_include && segment == "include" {
        return quote_string(segment);
    }

    if is_safe_unquoted_path_segment(segment) {
        segment.to_string()
    } else {
        quote_string(segment)
    }
}

fn is_safe_unquoted_path_segment(segment: &str) -> bool {
    if segment.is_empty() || segment.contains("//") {
        return false;
    }

    for ch in segment.chars() {
        if ch == '.'
            || ch == '$'
            || ch == '"'
            || ch == '{'
            || ch == '}'
            || ch == '['
            || ch == ']'
            || ch == ':'
            || ch == '='
            || ch == ','
            || ch == '#'
            || ch == '`'
            || ch == '^'
            || ch == '?'
            || ch == '!'
            || ch == '@'
            || ch == '*'
            || ch == '&'
            || ch == '\\'
            || ch.is_whitespace()
        {
            return false;
        }
    }

    true
}

fn quote_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                let code = ch as u32;
                out.push_str(&format!("\\u{:04X}", code));
            }
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

#[derive(Clone, Copy)]
enum PathMode {
    FieldKey,
    Substitution,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse_document(&mut self) -> Result<Document, ParseError> {
        self.skip_ws_comments_newlines();

        let root = match self.peek_char() {
            Some('{') => {
                let entries = self.parse_object_entries(Some('}'))?;
                Root::Object {
                    entries,
                    braced: true,
                }
            }
            Some('[') => Root::Array(self.parse_array_items()?),
            Some(_) => Root::Object {
                entries: self.parse_object_entries(None)?,
                braced: false,
            },
            None => Root::Object {
                entries: Vec::new(),
                braced: false,
            },
        };

        self.skip_ws_comments_newlines();
        if !self.is_eof() {
            return Err(self.error("unexpected trailing content"));
        }

        Ok(Document { root })
    }

    fn parse_object_entries(&mut self, terminator: Option<char>) -> Result<Vec<Entry>, ParseError> {
        if terminator == Some('}') {
            self.expect_char('{')?;
        }

        self.skip_ws_comments_newlines();

        let mut entries = Vec::new();
        loop {
            if self.is_object_end(terminator) {
                break;
            }

            entries.push(self.parse_entry()?);

            if self.is_object_end(terminator) {
                break;
            }

            let had_separator = self.consume_body_separator();
            if self.is_object_end(terminator) {
                break;
            }
            if !had_separator {
                return Err(self.error("expected a comma or newline between object entries"));
            }
        }

        if terminator == Some('}') {
            self.expect_char('}')?;
        }

        Ok(entries)
    }

    fn parse_array_items(&mut self) -> Result<Vec<Value>, ParseError> {
        self.expect_char('[')?;
        self.skip_ws_comments_newlines();

        let mut items = Vec::new();
        loop {
            if self.peek_char() == Some(']') {
                break;
            }

            items.push(self.parse_value()?);

            if self.peek_char() == Some(']') {
                break;
            }

            let had_separator = self.consume_body_separator();
            if self.peek_char() == Some(']') {
                break;
            }
            if !had_separator {
                return Err(self.error("expected a comma or newline between array elements"));
            }
        }

        self.expect_char(']')?;
        Ok(items)
    }

    fn parse_entry(&mut self) -> Result<Entry, ParseError> {
        self.skip_ws_comments_newlines();

        if self.starts_include_statement() {
            Ok(Entry::Include(self.parse_include()?))
        } else {
            Ok(Entry::Field(self.parse_field()?))
        }
    }

    fn parse_field(&mut self) -> Result<Field, ParseError> {
        let path = self.parse_path(PathMode::FieldKey)?;
        self.skip_ws_comments_newlines();

        let op = if self.starts_with("+=") {
            self.pos += 2;
            FieldOp::Append
        } else if self.starts_with("=") || self.starts_with(":") {
            self.pos += 1;
            FieldOp::Set
        } else if self.peek_char() == Some('{') {
            FieldOp::Set
        } else {
            return Err(self.error("expected '=', ':', '+=' or '{' after a field key"));
        };

        if self.peek_char() != Some('{') || matches!(op, FieldOp::Append) {
            self.skip_ws_comments_newlines();
        }

        let value = self.parse_value()?;
        Ok(Field { path, op, value })
    }

    fn parse_include(&mut self) -> Result<Include, ParseError> {
        self.expect_keyword("include")?;
        if !self.consume_ws_or_comment_separator() {
            return Err(self.error("expected whitespace after 'include'"));
        }
        self.skip_ws_comments_newlines();
        self.parse_include_target()
    }

    fn parse_include_target(&mut self) -> Result<Include, ParseError> {
        if self.starts_with("required") && self.is_boundary_after_keyword("required") {
            self.expect_keyword("required")?;
            self.skip_ws_comments_newlines();
            self.expect_char('(')?;
            self.skip_ws_comments_newlines();
            let inner = self.parse_include_target()?;
            self.skip_ws_comments_newlines();
            self.expect_char(')')?;
            if matches!(inner, Include::Required(_)) {
                return Err(self.error("nested required() includes are not valid"));
            }
            return Ok(Include::Required(Box::new(inner)));
        }

        if self.starts_with("file") && self.is_boundary_after_keyword("file") {
            return self.parse_include_function("file");
        }
        if self.starts_with("url") && self.is_boundary_after_keyword("url") {
            return self.parse_include_function("url");
        }
        if self.starts_with("classpath") && self.is_boundary_after_keyword("classpath") {
            return self.parse_include_function("classpath");
        }
        if self.peek_char() == Some('"') {
            return Ok(Include::Bare(self.parse_string_literal()?));
        }

        Err(self.error("expected a quoted include target or include function"))
    }

    fn parse_include_function(&mut self, kind: &str) -> Result<Include, ParseError> {
        self.expect_keyword(kind)?;
        self.skip_ws_comments_newlines();
        self.expect_char('(')?;
        self.skip_ws_comments_newlines();
        let path = self.parse_string_literal()?;
        self.skip_ws_comments_newlines();
        self.expect_char(')')?;

        Ok(match kind {
            "file" => Include::File(path),
            "url" => Include::Url(path),
            "classpath" => Include::Classpath(path),
            _ => unreachable!("unsupported include function"),
        })
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        let first = self.parse_value_part()?;
        let mut items = vec![ConcatItem {
            separator: String::new(),
            part: first,
        }];

        loop {
            let separator = self.consume_inline_whitespace();
            if self.starts_comment() || self.peek_char() == Some('\n') || self.is_value_terminator()
            {
                break;
            }
            if !self.can_start_value_part() {
                break;
            }

            items.push(ConcatItem {
                separator,
                part: self.parse_value_part()?,
            });
        }

        if items.len() == 1 {
            Ok(Value::Single(items.pop().unwrap().part))
        } else {
            Ok(Value::Concat(items))
        }
    }

    fn parse_value_part(&mut self) -> Result<ValuePart, ParseError> {
        match self.peek_char() {
            Some('{') => Ok(ValuePart::Object(self.parse_object_entries(Some('}'))?)),
            Some('[') => Ok(ValuePart::Array(self.parse_array_items()?)),
            Some('"') => Ok(ValuePart::Atom(Atom::String(self.parse_string_literal()?))),
            Some('$') if self.starts_with("${") => Ok(ValuePart::Atom(Atom::Substitution(
                self.parse_substitution()?,
            ))),
            Some(_) => Ok(ValuePart::Atom(self.parse_simple_atom()?)),
            None => Err(self.error("expected a value")),
        }
    }

    fn parse_simple_atom(&mut self) -> Result<Atom, ParseError> {
        if let Some(keyword) = self.match_keyword_atom() {
            self.pos += keyword.len();
            return Ok(match keyword {
                "null" => Atom::Null,
                _ => Atom::Boolean(keyword.to_string()),
            });
        }

        if let Some(number) = self.try_parse_number(false)? {
            return Ok(Atom::Number(number));
        }

        let text = self.parse_unquoted(false)?;
        Ok(Atom::Unquoted(text))
    }

    fn parse_substitution(&mut self) -> Result<Substitution, ParseError> {
        self.expect_str("${")?;
        let optional = if self.peek_char() == Some('?') {
            self.pos += 1;
            true
        } else {
            false
        };

        let _ = self.consume_inline_whitespace();
        let path = self.parse_path(PathMode::Substitution)?;
        let _ = self.consume_inline_whitespace();

        let list_hint = if self.starts_with("[]") {
            self.pos += 2;
            true
        } else {
            false
        };

        let _ = self.consume_inline_whitespace();
        self.expect_char('}')?;

        Ok(Substitution {
            optional,
            path,
            list_hint,
        })
    }

    fn parse_path(&mut self, mode: PathMode) -> Result<Vec<String>, ParseError> {
        let mut path = vec![self.parse_path_segment(mode)?];

        loop {
            let _ = self.consume_inline_whitespace();
            if self.peek_char() != Some('.') {
                break;
            }
            self.pos += 1;
            let _ = self.consume_inline_whitespace();
            path.push(self.parse_path_segment(mode)?);
        }

        Ok(path)
    }

    fn parse_path_segment(&mut self, mode: PathMode) -> Result<String, ParseError> {
        if !self.can_start_path_token() {
            return Err(self.error("expected a path expression"));
        }

        let mut segment = String::new();

        loop {
            segment.push_str(&self.parse_path_token()?);
            let whitespace = self.consume_inline_whitespace();

            if self.peek_char() == Some('.')
                || self.starts_comment()
                || self.peek_char() == Some('\n')
                || self.is_path_terminator(mode)
            {
                break;
            }

            if self.can_start_path_token() {
                segment.push_str(&whitespace);
                continue;
            }

            break;
        }

        Ok(segment)
    }

    fn parse_path_token(&mut self) -> Result<String, ParseError> {
        if self.peek_char() == Some('"') {
            return self.parse_string_literal()?.decode();
        }

        if let Some(keyword) = self.match_keyword_atom() {
            self.pos += keyword.len();
            return Ok(keyword.to_string());
        }

        if let Some(number) = self.try_parse_number(true)? {
            return Ok(number);
        }

        self.parse_unquoted(true)
    }

    fn try_parse_number(&mut self, stop_at_dot: bool) -> Result<Option<String>, ParseError> {
        let start = self.pos;

        if self.peek_char() == Some('-') {
            self.pos += 1;
        }

        match self.peek_char() {
            Some('0') => {
                self.pos += 1;
            }
            Some(ch) if ch.is_ascii_digit() => {
                self.pos += 1;
                while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
                    self.pos += 1;
                }
            }
            _ => {
                self.pos = start;
                return Ok(None);
            }
        }

        if !stop_at_dot
            && self.peek_char() == Some('.')
            && matches!(self.peek_nth_char(1), Some(ch) if ch.is_ascii_digit())
        {
            self.pos += 1;
            while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
                self.pos += 1;
            }
        }

        if matches!(self.peek_char(), Some('e' | 'E')) {
            let exponent_mark = self.pos;
            self.pos += 1;
            if matches!(self.peek_char(), Some('+' | '-')) {
                self.pos += 1;
            }
            let exponent_start = self.pos;
            while matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit()) {
                self.pos += 1;
            }
            if exponent_start == self.pos {
                self.pos = exponent_mark;
            }
        }

        let number = self.input[start..self.pos].to_string();
        Ok(Some(number))
    }

    fn parse_unquoted(&mut self, stop_at_dot: bool) -> Result<String, ParseError> {
        let start = self.pos;

        while let Some(ch) = self.peek_char() {
            if self.starts_comment() || ch == '\n' || ch.is_whitespace() {
                break;
            }
            if ch == '$'
                || ch == '"'
                || ch == '{'
                || ch == '}'
                || ch == '['
                || ch == ']'
                || ch == ':'
                || ch == '='
                || ch == ','
                || ch == '#'
                || ch == '`'
                || ch == '^'
                || ch == '?'
                || ch == '!'
                || ch == '@'
                || ch == '*'
                || ch == '&'
                || ch == '\\'
                || ch == '+'
                || (stop_at_dot && ch == '.')
            {
                break;
            }

            self.pos += ch.len_utf8();
        }

        if start == self.pos {
            return Err(self.error("expected an unquoted token"));
        }

        Ok(self.input[start..self.pos].to_string())
    }

    fn parse_string_literal(&mut self) -> Result<StringLiteral, ParseError> {
        if self.starts_with("\"\"\"") {
            self.parse_multiline_string()
        } else {
            self.parse_quoted_string()
        }
    }

    fn parse_quoted_string(&mut self) -> Result<StringLiteral, ParseError> {
        let start = self.pos;
        self.expect_char('"')?;

        while let Some(ch) = self.peek_char() {
            match ch {
                '"' => {
                    self.pos += 1;
                    return Ok(StringLiteral {
                        raw: self.input[start..self.pos].to_string(),
                    });
                }
                '\\' => {
                    self.pos += 1;
                    match self.peek_char() {
                        Some('"') | Some('\\') | Some('/') | Some('b') | Some('f') | Some('n')
                        | Some('r') | Some('t') => {
                            self.pos += 1;
                        }
                        Some('u') => {
                            self.pos += 1;
                            for _ in 0..4 {
                                match self.peek_char() {
                                    Some(c) if c.is_ascii_hexdigit() => self.pos += 1,
                                    _ => {
                                        return Err(self.error(
                                            "expected four hexadecimal digits in \\u escape",
                                        ));
                                    }
                                }
                            }
                        }
                        _ => return Err(self.error("invalid escape sequence in quoted string")),
                    }
                }
                '\n' => return Err(self.error("quoted strings cannot contain newlines")),
                _ => self.pos += ch.len_utf8(),
            }
        }

        Err(self.error("unterminated quoted string"))
    }

    fn parse_multiline_string(&mut self) -> Result<StringLiteral, ParseError> {
        let start = self.pos;
        self.expect_str("\"\"\"")?;

        loop {
            if self.is_eof() {
                return Err(self.error("unterminated multiline string"));
            }

            if self.peek_char() == Some('"') {
                let run = self.count_consecutive_quotes();
                if run >= 3 {
                    self.pos += run;
                    return Ok(StringLiteral {
                        raw: self.input[start..self.pos].to_string(),
                    });
                }
                self.pos += run;
            } else {
                self.pos += self.peek_char().unwrap().len_utf8();
            }
        }
    }

    fn count_consecutive_quotes(&self) -> usize {
        self.input[self.pos..]
            .chars()
            .take_while(|ch| *ch == '"')
            .count()
    }

    fn starts_include_statement(&self) -> bool {
        self.starts_with("include") && self.is_boundary_after_keyword("include")
    }

    fn match_keyword_atom(&self) -> Option<&'static str> {
        for keyword in ["true", "false", "null", "yes", "no", "on", "off"] {
            if self.starts_with(keyword) {
                return Some(keyword);
            }
        }

        None
    }

    fn is_path_terminator(&self, mode: PathMode) -> bool {
        match mode {
            PathMode::FieldKey => {
                self.starts_with("+=")
                    || self.starts_with("=")
                    || self.starts_with(":")
                    || self.peek_char() == Some('{')
            }
            PathMode::Substitution => self.starts_with("[]") || self.peek_char() == Some('}'),
        }
    }

    fn can_start_value_part(&self) -> bool {
        matches!(self.peek_char(), Some('{' | '[' | '"'))
            || self.starts_with("${")
            || self.can_start_simple_atom()
    }

    fn can_start_simple_atom(&self) -> bool {
        if self.match_keyword_atom().is_some() {
            return true;
        }
        if matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit() || ch == '-') {
            return true;
        }
        self.can_start_unquoted(false)
    }

    fn can_start_path_token(&self) -> bool {
        if self.peek_char() == Some('"') {
            return true;
        }
        if self.match_keyword_atom().is_some() {
            return true;
        }
        if matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit() || ch == '-') {
            return true;
        }
        self.can_start_unquoted(true)
    }

    fn can_start_unquoted(&self, stop_at_dot: bool) -> bool {
        match self.peek_char() {
            Some(ch) => {
                !ch.is_whitespace()
                    && ch != '\n'
                    && ch != '$'
                    && ch != '"'
                    && ch != '{'
                    && ch != '}'
                    && ch != '['
                    && ch != ']'
                    && ch != ':'
                    && ch != '='
                    && ch != ','
                    && ch != '#'
                    && ch != '`'
                    && ch != '^'
                    && ch != '?'
                    && ch != '!'
                    && ch != '@'
                    && ch != '*'
                    && ch != '&'
                    && ch != '\\'
                    && ch != '+'
                    && !(stop_at_dot && ch == '.')
                    && !self.starts_comment()
            }
            None => false,
        }
    }

    fn is_value_terminator(&self) -> bool {
        matches!(self.peek_char(), Some('}' | ']' | ',') | None)
    }

    fn is_object_end(&self, terminator: Option<char>) -> bool {
        match terminator {
            Some(ch) => self.peek_char() == Some(ch),
            None => self.is_eof(),
        }
    }

    fn consume_body_separator(&mut self) -> bool {
        let mut saw_newline = false;

        loop {
            let _ = self.consume_inline_whitespace();

            if self.peek_char() == Some(',') {
                self.pos += 1;
                self.skip_ws_comments_newlines();
                return true;
            }

            if self.starts_comment() {
                self.skip_comment();
                continue;
            }

            if self.peek_char() == Some('\n') {
                saw_newline = true;
                self.pos += 1;
                continue;
            }

            break;
        }

        saw_newline
    }

    fn consume_ws_or_comment_separator(&mut self) -> bool {
        let before = self.pos;
        let _ = self.consume_inline_whitespace();
        if self.starts_comment() || self.peek_char() == Some('\n') {
            self.skip_ws_comments_newlines();
        }
        self.pos > before
    }

    fn consume_inline_whitespace(&mut self) -> String {
        let start = self.pos;
        while matches!(self.peek_char(), Some(ch) if is_inline_whitespace(ch)) {
            self.pos += self.peek_char().unwrap().len_utf8();
        }
        self.input[start..self.pos].to_string()
    }

    fn skip_ws_comments_newlines(&mut self) {
        loop {
            let before = self.pos;

            let _ = self.consume_inline_whitespace();

            if self.starts_comment() {
                self.skip_comment();
            }

            while self.peek_char() == Some('\n') {
                self.pos += 1;
                let _ = self.consume_inline_whitespace();
                if self.starts_comment() {
                    self.skip_comment();
                }
            }

            if self.pos == before {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        if self.peek_char() == Some('#') {
            self.pos += 1;
        } else if self.starts_with("//") {
            self.pos += 2;
        } else {
            return;
        }

        while let Some(ch) = self.peek_char() {
            if ch == '\n' {
                break;
            }
            self.pos += ch.len_utf8();
        }
    }

    fn starts_comment(&self) -> bool {
        self.peek_char() == Some('#') || self.starts_with("//")
    }

    fn expect_char(&mut self, expected: char) -> Result<(), ParseError> {
        match self.peek_char() {
            Some(ch) if ch == expected => {
                self.pos += ch.len_utf8();
                Ok(())
            }
            _ => Err(self.error(format!("expected '{}'", expected))),
        }
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<(), ParseError> {
        if self.starts_with(keyword) {
            self.pos += keyword.len();
            Ok(())
        } else {
            Err(self.error(format!("expected '{}'", keyword)))
        }
    }

    fn expect_str(&mut self, expected: &str) -> Result<(), ParseError> {
        if self.starts_with(expected) {
            self.pos += expected.len();
            Ok(())
        } else {
            Err(self.error(format!("expected '{}'", expected)))
        }
    }

    fn is_boundary_after_keyword(&self, keyword: &str) -> bool {
        let Some(next) = self.input[self.pos + keyword.len()..].chars().next() else {
            return true;
        };

        next.is_whitespace()
            || next == '.'
            || next == ':'
            || next == '='
            || next == '+'
            || next == '{'
            || next == '['
            || next == '('
            || next == ')'
            || next == '}'
            || next == ','
            || next == '#'
            || next == '"'
            || next == '/'
    }

    fn starts_with(&self, value: &str) -> bool {
        self.input[self.pos..].starts_with(value)
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn peek_nth_char(&self, offset: usize) -> Option<char> {
        self.input[self.pos..].chars().nth(offset)
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError::new(self.input, self.pos, message)
    }
}

impl StringLiteral {
    fn decode(&self) -> Result<String, ParseError> {
        if self.raw.starts_with("\"\"\"") {
            return Ok(decode_multiline_string(&self.raw));
        }
        decode_quoted_string(&self.raw)
    }
}

fn decode_multiline_string(raw: &str) -> String {
    let trailing_quotes = raw.chars().rev().take_while(|ch| *ch == '"').count();
    let mut out = raw[3..raw.len() - trailing_quotes].to_string();
    out.push_str(&"\"".repeat(trailing_quotes.saturating_sub(3)));
    out
}

fn decode_quoted_string(raw: &str) -> Result<String, ParseError> {
    let mut out = String::new();
    let mut chars = raw[1..raw.len() - 1].chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some('/') => out.push('/'),
            Some('b') => out.push('\u{08}'),
            Some('f') => out.push('\u{0c}'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('u') => {
                let mut hex = String::new();
                for _ in 0..4 {
                    if let Some(digit) = chars.next() {
                        hex.push(digit);
                    }
                }
                let code = u32::from_str_radix(&hex, 16).map_err(|_| ParseError {
                    message: "invalid \\u escape".to_string(),
                    line: 1,
                    column: 1,
                })?;
                if let Some(decoded) = char::from_u32(code) {
                    out.push(decoded);
                } else {
                    return Err(ParseError {
                        message: "invalid Unicode scalar value".to_string(),
                        line: 1,
                        column: 1,
                    });
                }
            }
            _ => {
                return Err(ParseError {
                    message: "invalid escape sequence".to_string(),
                    line: 1,
                    column: 1,
                });
            }
        }
    }

    Ok(out)
}

fn is_inline_whitespace(ch: char) -> bool {
    ch != '\n' && (ch.is_whitespace() || ch == '\u{feff}')
}

#[cfg(test)]
mod tests {
    use super::{CommaStyle, FormatOptions, format_hocon, format_hocon_with_options};

    #[test]
    fn formats_implicit_root_object_and_nested_values() {
        let input = r#"foo:{bar=1,baz:[2,3]}"#;
        let expected = r#"foo = {
  bar = 1
  baz = [
    2
    3
  ]
}
"#;

        assert_eq!(format_hocon(input).unwrap(), expected);
    }

    #[test]
    fn preserves_literal_concatenation_spacing() {
        let input = "message = foo  bar";
        let expected = "message = foo  bar\n";

        assert_eq!(format_hocon(input).unwrap(), expected);
    }

    #[test]
    fn formats_includes_substitutions_and_append() {
        let input = r#"
include required(file("base.conf"))
foo.bar."baz qux"+=[1,2]
ref=${?ENV_VAR}
"#;
        let expected = r#"include required(file("base.conf"))
foo.bar."baz qux" += [
  1
  2
]
ref = ${?ENV_VAR}
"#;

        assert_eq!(format_hocon(input).unwrap(), expected);
    }

    #[test]
    fn formats_object_and_array_concatenation() {
        let input = r#"
merged = { x = 1 }{ y = 2 }
arrays = [1,2] [3,4]
"#;
        let expected = r#"merged = {
  x = 1
}{
  y = 2
}
arrays = [
  1
  2
] [
  3
  4
]
"#;

        assert_eq!(format_hocon(input).unwrap(), expected);
    }

    #[test]
    fn supports_concatenation_inside_arrays() {
        let input = r#"[foo bar, { a = 1 } { b = 2 }]"#;
        let expected = r#"[
  foo bar
  {
    a = 1
  } {
    b = 2
  }
]
"#;

        assert_eq!(format_hocon(input).unwrap(), expected);
    }

    #[test]
    fn formats_path_segments_with_spaces_canonically() {
        let input = r#"a b.c:1"#;
        let expected = "\"a b\".c = 1\n";

        assert_eq!(format_hocon(input).unwrap(), expected);
    }

    #[test]
    fn supports_environment_list_substitutions() {
        let input = r#"list = ${MY_LIST[]}"#;
        let expected = "list = ${MY_LIST[]}\n";

        assert_eq!(format_hocon(input).unwrap(), expected);
    }

    #[test]
    fn accepts_numbers_followed_by_unquoted_concatenation() {
        let input = r#"value = 1.foo"#;
        let expected = "value = 1.foo\n";

        assert_eq!(format_hocon(input).unwrap(), expected);
    }

    #[test]
    fn preserves_explicit_root_object() {
        let input = r#"{ foo = 1, bar = { baz = true } }"#;
        let expected = r#"{
  foo = 1
  bar = {
    baz = true
  }
}
"#;

        assert_eq!(format_hocon(input).unwrap(), expected);
    }

    #[test]
    fn accepts_numeric_path_components() {
        let input = "3.14:42";
        let expected = "3.14 = 42\n";

        assert_eq!(format_hocon(input).unwrap(), expected);
    }

    #[test]
    fn formats_with_commas_between_elements() {
        let input = r#"foo:{bar=1,baz:[2,3]}"#;
        let expected = r#"foo = {
  bar = 1,
  baz = [
    2,
    3
  ]
}
"#;

        assert_eq!(
            format_hocon_with_options(
                input,
                FormatOptions {
                    comma_style: CommaStyle::Commas,
                },
            )
            .unwrap(),
            expected
        );
    }

    #[test]
    fn formats_with_trailing_commas() {
        let input = r#"{ foo = 1, bar = [2,3] }"#;
        let expected = r#"{
  foo = 1,
  bar = [
    2,
    3,
  ],
}
"#;

        assert_eq!(
            format_hocon_with_options(
                input,
                FormatOptions {
                    comma_style: CommaStyle::Trailing,
                },
            )
            .unwrap(),
            expected
        );
    }
}
