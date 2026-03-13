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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatOptions {
    pub comma_style: CommaStyle,
    pub max_width: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            comma_style: CommaStyle::None,
            max_width: 80,
        }
    }
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
struct ObjectValue {
    entries: Vec<Entry>,
    prefer_multiline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArrayValue {
    items: Vec<ArrayItem>,
    prefer_multiline: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Root {
    Object { object: ObjectValue, braced: bool },
    Array(ArrayValue),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    Field(Field),
    Include(IncludeStmt),
    Comment(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ArrayItem {
    Value {
        value: Value,
        trailing_comment: Option<String>,
    },
    Comment(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IncludeStmt {
    include: Include,
    trailing_comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    path: Vec<String>,
    op: FieldOp,
    value: Value,
    trailing_comment: Option<String>,
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
    Object(ObjectValue),
    Array(ArrayValue),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConcatKind {
    Simple,
    Array,
    Object,
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
            Root::Object { object, braced } => {
                if *braced {
                    format!("{}\n", format_object(object, 0, 0, options))
                } else if object.entries.is_empty() {
                    String::new()
                } else {
                    format_root_entries(&object.entries, options)
                }
            }
            Root::Array(array) => format!("{}\n", format_array(array, 0, 0, options)),
        }
    }
}

fn format_root_entries(entries: &[Entry], options: FormatOptions) -> String {
    let mut out = String::new();
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            let prev_non_comment = !matches!(entries[index - 1], Entry::Comment(_));
            let current_non_comment = !matches!(entry, Entry::Comment(_));
            if prev_non_comment && current_non_comment {
                out.push_str("\n\n");
            } else {
                out.push('\n');
            }
        }
        out.push_str(&format_entry(entry, 0, options));
    }
    out.push('\n');
    out
}

fn format_object(
    object: &ObjectValue,
    indent: usize,
    current_column: usize,
    options: FormatOptions,
) -> String {
    if let Some(inline) = format_object_inline(object, options) {
        if current_column + text_width(&inline) <= options.max_width {
            return inline;
        }
    }

    format_object_multiline(object, indent, options)
}

fn format_object_multiline(object: &ObjectValue, indent: usize, options: FormatOptions) -> String {
    if object.entries.is_empty() {
        return format!("{{\n{}}}", " ".repeat(indent));
    }

    let value_indices: Vec<usize> = object
        .entries
        .iter()
        .enumerate()
        .filter_map(|(idx, entry)| (!matches!(entry, Entry::Comment(_))).then_some(idx))
        .collect();

    let mut out = String::new();
    out.push_str("{\n");
    for (index, entry) in object.entries.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let entry_indent = indent + 2;
        out.push_str(&" ".repeat(entry_indent));
        out.push_str(&format_entry(entry, entry_indent, options));
        if should_add_comma_for_item(index, &value_indices, options.comma_style) {
            out.push(',');
        }
    }
    out.push('\n');
    out.push_str(&" ".repeat(indent));
    out.push('}');
    out
}

fn format_object_inline(object: &ObjectValue, options: FormatOptions) -> Option<String> {
    if object.prefer_multiline {
        return None;
    }

    let mut parts = Vec::new();
    for entry in &object.entries {
        parts.push(format_entry_inline(entry, options)?);
    }

    if parts.is_empty() {
        Some("{}".to_string())
    } else {
        Some(format!("{{ {} }}", parts.join(", ")))
    }
}

fn format_array(
    array: &ArrayValue,
    indent: usize,
    current_column: usize,
    options: FormatOptions,
) -> String {
    if let Some(inline) = format_array_inline(array, options) {
        if current_column + text_width(&inline) <= options.max_width {
            return inline;
        }
    }

    format_array_multiline(array, indent, options)
}

fn format_array_multiline(array: &ArrayValue, indent: usize, options: FormatOptions) -> String {
    if array.items.is_empty() {
        return format!("[\n{}]", " ".repeat(indent));
    }

    let value_indices: Vec<usize> = array
        .items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| matches!(item, ArrayItem::Value { .. }).then_some(idx))
        .collect();

    let mut out = String::new();
    out.push_str("[\n");
    for (index, item) in array.items.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        let item_indent = indent + 2;
        out.push_str(&" ".repeat(item_indent));
        match item {
            ArrayItem::Value {
                value,
                trailing_comment,
            } => {
                out.push_str(&format_value(value, item_indent, item_indent, options));
                if let Some(comment) = trailing_comment {
                    out.push_str(comment);
                }
                if should_add_comma_for_item(index, &value_indices, options.comma_style) {
                    out.push(',');
                }
            }
            ArrayItem::Comment(comment) => out.push_str(comment),
        }
    }
    out.push('\n');
    out.push_str(&" ".repeat(indent));
    out.push(']');
    out
}

fn format_array_inline(array: &ArrayValue, options: FormatOptions) -> Option<String> {
    if array.prefer_multiline {
        return None;
    }

    let mut parts = Vec::new();
    for item in &array.items {
        parts.push(format_array_item_inline(item, options)?);
    }

    if parts.is_empty() {
        Some("[]".to_string())
    } else {
        Some(format!("[ {} ]", parts.join(", ")))
    }
}

fn format_entry(entry: &Entry, indent: usize, options: FormatOptions) -> String {
    match entry {
        Entry::Field(field) => {
            let operator = match field.op {
                FieldOp::Set => "=",
                FieldOp::Append => "+=",
            };
            let prefix = format!("{} {} ", format_path(&field.path, true), operator);
            let mut out = prefix.clone();
            out.push_str(&format_value(
                &field.value,
                indent,
                indent + text_width(&prefix),
                options,
            ));
            if let Some(comment) = &field.trailing_comment {
                out.push_str(comment);
            }
            out
        }
        Entry::Include(include) => {
            let mut out = format_include(&include.include);
            if let Some(comment) = &include.trailing_comment {
                out.push_str(comment);
            }
            out
        }
        Entry::Comment(comment) => comment.clone(),
    }
}

fn format_entry_inline(entry: &Entry, options: FormatOptions) -> Option<String> {
    match entry {
        Entry::Field(field) => {
            if field.trailing_comment.is_some() {
                return None;
            }

            let operator = match field.op {
                FieldOp::Set => "=",
                FieldOp::Append => "+=",
            };
            let mut out = format!("{} {} ", format_path(&field.path, true), operator);
            out.push_str(&format_value_inline(&field.value, options)?);
            Some(out)
        }
        Entry::Include(include) => {
            if include.trailing_comment.is_some() {
                return None;
            }
            Some(format_include(&include.include))
        }
        Entry::Comment(_) => None,
    }
}

fn format_array_item_inline(item: &ArrayItem, options: FormatOptions) -> Option<String> {
    match item {
        ArrayItem::Value {
            value,
            trailing_comment,
        } => {
            if trailing_comment.is_some() {
                return None;
            }
            format_value_inline(value, options)
        }
        ArrayItem::Comment(_) => None,
    }
}

fn should_add_comma_for_item(
    index: usize,
    value_indices: &[usize],
    comma_style: CommaStyle,
) -> bool {
    if !value_indices.contains(&index) {
        return false;
    }

    match comma_style {
        CommaStyle::None => false,
        CommaStyle::Commas => value_indices.last().copied() != Some(index),
        CommaStyle::Trailing => true,
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

fn format_value(
    value: &Value,
    indent: usize,
    current_column: usize,
    options: FormatOptions,
) -> String {
    match value {
        Value::Single(part) => format_value_part(part, indent, current_column, options),
        Value::Concat(items) => {
            if let Some(inline) = format_value_inline(value, options) {
                if current_column + text_width(&inline) <= options.max_width {
                    return inline;
                }
            }

            let mut out = String::new();
            for item in items {
                out.push_str(&item.separator);
                let part_column = current_column_after(current_column, &out);
                out.push_str(&format_value_part(&item.part, indent, part_column, options));
            }
            out
        }
    }
}

fn format_value_inline(value: &Value, options: FormatOptions) -> Option<String> {
    match value {
        Value::Single(part) => format_value_part_inline(part, options),
        Value::Concat(items) => {
            let mut out = String::new();
            for item in items {
                out.push_str(&item.separator);
                out.push_str(&format_value_part_inline(&item.part, options)?);
            }
            Some(out)
        }
    }
}

fn format_value_part(
    part: &ValuePart,
    indent: usize,
    current_column: usize,
    options: FormatOptions,
) -> String {
    match part {
        ValuePart::Object(object) => format_object(object, indent, current_column, options),
        ValuePart::Array(array) => format_array(array, indent, current_column, options),
        ValuePart::Atom(atom) => format_atom(atom),
    }
}

fn format_value_part_inline(part: &ValuePart, options: FormatOptions) -> Option<String> {
    match part {
        ValuePart::Object(object) => format_object_inline(object, options),
        ValuePart::Array(array) => format_array_inline(array, options),
        ValuePart::Atom(atom) => Some(format_atom(atom)),
    }
}

fn current_column_after(start_column: usize, text: &str) -> usize {
    match text.rsplit_once('\n') {
        Some((_, tail)) => text_width(tail),
        None => start_column + text_width(text),
    }
}

fn text_width(text: &str) -> usize {
    text.chars().count()
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

fn classify_value_part_for_concat(part: &ValuePart) -> Option<ConcatKind> {
    match part {
        ValuePart::Object(_) => Some(ConcatKind::Object),
        ValuePart::Array(_) => Some(ConcatKind::Array),
        ValuePart::Atom(Atom::Substitution(_)) => None,
        ValuePart::Atom(_) => Some(ConcatKind::Simple),
    }
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum SeparatorState {
    Start,
    SawComma,
    SawNewline,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse_document(&mut self) -> Result<Document, ParseError> {
        self.skip_layout_without_comments();

        let root = match self.peek_char() {
            Some('{') => {
                let object = self.parse_object_entries(Some('}'))?;
                Root::Object {
                    object,
                    braced: true,
                }
            }
            Some('[') => Root::Array(self.parse_array_items()?),
            Some(_) => Root::Object {
                object: self.parse_object_entries(None)?,
                braced: false,
            },
            None => Root::Object {
                object: ObjectValue {
                    entries: Vec::new(),
                    prefer_multiline: false,
                },
                braced: false,
            },
        };

        let has_implicit_root_value = match &root {
            Root::Object {
                object,
                braced: false,
            } => object
                .entries
                .iter()
                .any(|entry| !matches!(entry, Entry::Comment(_))),
            _ => true,
        };
        if !has_implicit_root_value {
            return Err(self.error("empty documents are not valid"));
        }

        self.skip_layout_without_comments();
        if !self.is_eof() {
            return Err(self.error("unexpected trailing content"));
        }

        Ok(Document { root })
    }

    fn parse_object_entries(
        &mut self,
        terminator: Option<char>,
    ) -> Result<ObjectValue, ParseError> {
        if terminator == Some('}') {
            self.expect_char('{')?;
        }
        let content_start = self.pos;

        self.skip_layout_without_comments();

        let mut entries = Vec::new();
        loop {
            self.collect_standalone_comments_into_entries(&mut entries);

            if self.is_object_end(terminator) {
                break;
            }

            entries.push(self.parse_entry()?);
            if let Some(comment) = self.consume_inline_comment_suffix() {
                Self::attach_trailing_comment_to_entry(entries.last_mut().unwrap(), comment);
            }

            if self.is_object_end(terminator) {
                break;
            }

            let had_separator = self.consume_body_separator_into_entries(&mut entries)?;
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

        Ok(ObjectValue {
            entries,
            prefer_multiline: self.input[content_start..self.pos].contains('\n'),
        })
    }

    fn parse_array_items(&mut self) -> Result<ArrayValue, ParseError> {
        self.expect_char('[')?;
        let content_start = self.pos;
        self.skip_layout_without_comments();

        let mut items = Vec::new();
        loop {
            self.collect_standalone_comments_into_array_items(&mut items);

            if self.peek_char() == Some(']') {
                break;
            }

            items.push(ArrayItem::Value {
                value: self.parse_value()?,
                trailing_comment: None,
            });
            if let Some(comment) = self.consume_inline_comment_suffix() {
                Self::attach_trailing_comment_to_array_item(items.last_mut().unwrap(), comment);
            }

            if self.peek_char() == Some(']') {
                break;
            }

            let had_separator = self.consume_body_separator_into_array_items(&mut items)?;
            if self.peek_char() == Some(']') {
                break;
            }
            if !had_separator {
                return Err(self.error("expected a comma or newline between array elements"));
            }
        }

        self.expect_char(']')?;
        Ok(ArrayValue {
            items,
            prefer_multiline: self.input[content_start..self.pos].contains('\n'),
        })
    }

    fn parse_entry(&mut self) -> Result<Entry, ParseError> {
        self.skip_layout_without_comments();

        if self.starts_include_statement() {
            Ok(Entry::Include(IncludeStmt {
                include: self.parse_include()?,
                trailing_comment: None,
            }))
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
        Ok(Field {
            path,
            op,
            value,
            trailing_comment: None,
        })
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
            let before_separator = self.pos;
            let separator = self.consume_inline_whitespace();
            if self.starts_comment() || self.peek_char() == Some('\n') || self.is_value_terminator()
            {
                self.pos = before_separator;
                break;
            }
            if !self.can_start_value_part() {
                self.pos = before_separator;
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
            let mut concat_kind = None;
            for item in &items {
                let Some(kind) = classify_value_part_for_concat(&item.part) else {
                    continue;
                };

                match concat_kind {
                    Some(existing) if existing != kind => {
                        return Err(self.error("invalid mixed-type value concatenation"));
                    }
                    Some(_) => {}
                    None => concat_kind = Some(kind),
                }
            }

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
                ch if ch.is_control() => {
                    return Err(self.error("quoted strings cannot contain control characters"));
                }
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

    fn consume_body_separator_into_entries(
        &mut self,
        entries: &mut Vec<Entry>,
    ) -> Result<bool, ParseError> {
        let mut state = SeparatorState::Start;

        loop {
            let _ = self.consume_inline_whitespace();

            if self.peek_char() == Some(',') {
                if state != SeparatorState::Start {
                    return Err(self.error("unexpected comma between object entries"));
                }
                self.pos += 1;
                state = SeparatorState::SawComma;
                continue;
            }

            if self.starts_comment() {
                entries.push(Entry::Comment(self.consume_comment_text()));
                continue;
            }

            if self.peek_char() == Some('\n') {
                self.pos += 1;
                if state == SeparatorState::Start {
                    state = SeparatorState::SawNewline;
                }
                continue;
            }

            break;
        }

        Ok(state != SeparatorState::Start)
    }

    fn consume_body_separator_into_array_items(
        &mut self,
        items: &mut Vec<ArrayItem>,
    ) -> Result<bool, ParseError> {
        let mut state = SeparatorState::Start;

        loop {
            let _ = self.consume_inline_whitespace();

            if self.peek_char() == Some(',') {
                if state != SeparatorState::Start {
                    return Err(self.error("unexpected comma between array elements"));
                }
                self.pos += 1;
                state = SeparatorState::SawComma;
                continue;
            }

            if self.starts_comment() {
                items.push(ArrayItem::Comment(self.consume_comment_text()));
                continue;
            }

            if self.peek_char() == Some('\n') {
                self.pos += 1;
                if state == SeparatorState::Start {
                    state = SeparatorState::SawNewline;
                }
                continue;
            }

            break;
        }

        Ok(state != SeparatorState::Start)
    }

    fn collect_standalone_comments_into_entries(&mut self, entries: &mut Vec<Entry>) {
        loop {
            self.skip_layout_without_comments();
            if !self.starts_comment() {
                break;
            }
            entries.push(Entry::Comment(self.consume_comment_text()));
            self.skip_layout_without_comments();
            if self.peek_char() == Some('\n') {
                self.pos += 1;
            }
        }
    }

    fn collect_standalone_comments_into_array_items(&mut self, items: &mut Vec<ArrayItem>) {
        loop {
            self.skip_layout_without_comments();
            if !self.starts_comment() {
                break;
            }
            items.push(ArrayItem::Comment(self.consume_comment_text()));
            self.skip_layout_without_comments();
            if self.peek_char() == Some('\n') {
                self.pos += 1;
            }
        }
    }

    fn consume_inline_comment_suffix(&mut self) -> Option<String> {
        let before = self.pos;
        let ws = self.consume_inline_whitespace();
        if self.starts_comment() {
            return Some(format!("{}{}", ws, self.consume_comment_text()));
        }
        self.pos = before;
        None
    }

    fn attach_trailing_comment_to_entry(entry: &mut Entry, comment: String) {
        match entry {
            Entry::Field(field) => field.trailing_comment = Some(comment),
            Entry::Include(include) => include.trailing_comment = Some(comment),
            Entry::Comment(_) => {}
        }
    }

    fn attach_trailing_comment_to_array_item(item: &mut ArrayItem, comment: String) {
        match item {
            ArrayItem::Value {
                trailing_comment, ..
            } => *trailing_comment = Some(comment),
            ArrayItem::Comment(_) => {}
        }
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

    fn consume_comment_text(&mut self) -> String {
        let start = self.pos;
        self.skip_comment();
        self.input[start..self.pos].to_string()
    }

    fn skip_layout_without_comments(&mut self) {
        loop {
            let before = self.pos;
            let _ = self.consume_inline_whitespace();
            while self.peek_char() == Some('\n') {
                self.pos += 1;
                let _ = self.consume_inline_whitespace();
            }
            if self.pos == before {
                break;
            }
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
