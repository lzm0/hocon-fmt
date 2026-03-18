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
        let (line, column) = line_column_at(input, offset);

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
    Comment(StandaloneComment),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ArrayItem {
    Value {
        leading_blank_line: bool,
        value: Value,
        trailing_comment: Option<TrailingComment>,
    },
    Comment(StandaloneComment),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrailingComment {
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StandaloneComment {
    text: String,
    leading_blank_line: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IncludeStmt {
    leading_blank_line: bool,
    include: Include,
    trailing_comment: Option<TrailingComment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    leading_blank_line: bool,
    path: Vec<String>,
    op: FieldOp,
    value: Value,
    trailing_comment: Option<TrailingComment>,
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

trait CollectionItem {
    fn leading_blank_line(&self) -> bool;
    fn allows_comma(&self) -> bool;
    fn render(&self, indent: usize, add_comma: bool, options: FormatOptions) -> String;
    fn render_inline(&self, options: FormatOptions) -> Option<String>;
}

trait BodyItem {
    fn comment(comment: StandaloneComment) -> Self;
    fn attach_trailing_comment(&mut self, comment: TrailingComment);
}

impl Entry {
    fn set_leading_blank_line(&mut self, leading_blank_line: bool) {
        match self {
            Entry::Field(field) => field.leading_blank_line = leading_blank_line,
            Entry::Include(include) => include.leading_blank_line = leading_blank_line,
            Entry::Comment(comment) => comment.leading_blank_line = leading_blank_line,
        }
    }

    fn attach_trailing_comment(&mut self, comment: TrailingComment) {
        match self {
            Entry::Field(field) => field.trailing_comment = Some(comment),
            Entry::Include(include) => include.trailing_comment = Some(comment),
            Entry::Comment(_) => {}
        }
    }
}

impl CollectionItem for Entry {
    fn leading_blank_line(&self) -> bool {
        match self {
            Entry::Field(field) => field.leading_blank_line,
            Entry::Include(include) => include.leading_blank_line,
            Entry::Comment(comment) => comment.leading_blank_line,
        }
    }

    fn allows_comma(&self) -> bool {
        !matches!(self, Entry::Comment(_))
    }

    fn render(&self, indent: usize, add_comma: bool, options: FormatOptions) -> String {
        match self {
            Entry::Field(field) => format_field(field, indent, add_comma, options),
            Entry::Include(include) => format_include_stmt(include, add_comma),
            Entry::Comment(comment) => comment.text.clone(),
        }
    }

    fn render_inline(&self, options: FormatOptions) -> Option<String> {
        match self {
            Entry::Field(field) => format_field_inline(field, options),
            Entry::Include(include) => format_include_stmt_inline(include),
            Entry::Comment(_) => None,
        }
    }
}

impl BodyItem for Entry {
    fn comment(comment: StandaloneComment) -> Self {
        Entry::Comment(comment)
    }

    fn attach_trailing_comment(&mut self, comment: TrailingComment) {
        Entry::attach_trailing_comment(self, comment);
    }
}

impl ArrayItem {
    fn attach_trailing_comment(&mut self, comment: TrailingComment) {
        match self {
            ArrayItem::Value {
                trailing_comment, ..
            } => *trailing_comment = Some(comment),
            ArrayItem::Comment(_) => {}
        }
    }
}

impl CollectionItem for ArrayItem {
    fn leading_blank_line(&self) -> bool {
        match self {
            ArrayItem::Value {
                leading_blank_line, ..
            } => *leading_blank_line,
            ArrayItem::Comment(comment) => comment.leading_blank_line,
        }
    }

    fn allows_comma(&self) -> bool {
        matches!(self, ArrayItem::Value { .. })
    }

    fn render(&self, indent: usize, add_comma: bool, options: FormatOptions) -> String {
        match self {
            ArrayItem::Value {
                value,
                trailing_comment,
                ..
            } => {
                let mut out = format_value(value, indent, indent, options);
                push_trailing_comment(&mut out, trailing_comment.as_ref(), add_comma);
                out
            }
            ArrayItem::Comment(comment) => comment.text.clone(),
        }
    }

    fn render_inline(&self, options: FormatOptions) -> Option<String> {
        match self {
            ArrayItem::Value {
                value,
                trailing_comment,
                ..
            } => {
                if trailing_comment.is_some() {
                    return None;
                }
                format_value_inline(value, options)
            }
            ArrayItem::Comment(_) => None,
        }
    }
}

impl BodyItem for ArrayItem {
    fn comment(comment: StandaloneComment) -> Self {
        ArrayItem::Comment(comment)
    }

    fn attach_trailing_comment(&mut self, comment: TrailingComment) {
        ArrayItem::attach_trailing_comment(self, comment);
    }
}

impl Field {
    fn operator(&self) -> &'static str {
        match self.op {
            FieldOp::Set => "=",
            FieldOp::Append => "+=",
        }
    }

    fn prefix(&self) -> String {
        format!("{} {} ", format_path(&self.path, true), self.operator())
    }
}

impl Include {
    fn render(&self) -> String {
        format!("include {}", self.target())
    }

    fn target(&self) -> String {
        match self {
            Include::Bare(path) => path.raw.clone(),
            Include::File(path) => format!("file({})", path.raw),
            Include::Url(path) => format!("url({})", path.raw),
            Include::Classpath(path) => format!("classpath({})", path.raw),
            Include::Required(inner) => format!("required({})", inner.target()),
        }
    }
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
        push_multiline_item_separator(&mut out, index, entry.leading_blank_line());
        out.push_str(&entry.render(0, false, options));
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
    if object.entries.is_empty() {
        return "{}".to_string();
    }

    if let Some(inline) = format_object_inline(object, options) {
        if current_column + text_width(&inline) <= options.max_width {
            return inline;
        }
    }

    format_object_multiline(object, indent, options)
}

fn format_object_multiline(object: &ObjectValue, indent: usize, options: FormatOptions) -> String {
    format_delimited_multiline(&object.entries, indent, "{", "}", options)
}

fn format_object_inline(object: &ObjectValue, options: FormatOptions) -> Option<String> {
    if object.prefer_multiline {
        return None;
    }

    format_delimited_inline(&object.entries, "{}", "{ ", " }", options)
}

fn format_array(
    array: &ArrayValue,
    indent: usize,
    current_column: usize,
    options: FormatOptions,
) -> String {
    if array.items.is_empty() {
        return "[]".to_string();
    }

    if let Some(inline) = format_array_inline(array, options) {
        if current_column + text_width(&inline) <= options.max_width {
            return inline;
        }
    }

    format_array_multiline(array, indent, options)
}

fn format_array_multiline(array: &ArrayValue, indent: usize, options: FormatOptions) -> String {
    format_delimited_multiline(&array.items, indent, "[", "]", options)
}

fn format_array_inline(array: &ArrayValue, options: FormatOptions) -> Option<String> {
    if array.prefer_multiline {
        return None;
    }

    format_delimited_inline(&array.items, "[]", "[", "]", options)
}

fn format_delimited_multiline<T: CollectionItem>(
    items: &[T],
    indent: usize,
    open: &str,
    close: &str,
    options: FormatOptions,
) -> String {
    let last_comma_index = items.iter().rposition(|item| item.allows_comma());

    let mut out = String::new();
    out.push_str(open);
    out.push('\n');
    for (index, item) in items.iter().enumerate() {
        push_multiline_item_separator(&mut out, index, item.leading_blank_line());
        let item_indent = indent + 2;
        let add_comma = should_add_comma(
            item.allows_comma(),
            Some(index) == last_comma_index,
            options.comma_style,
        );
        out.push_str(&" ".repeat(item_indent));
        out.push_str(&item.render(item_indent, add_comma, options));
    }
    out.push('\n');
    out.push_str(&" ".repeat(indent));
    out.push_str(close);
    out
}

fn format_delimited_inline<T: CollectionItem>(
    items: &[T],
    empty: &str,
    open: &str,
    close: &str,
    options: FormatOptions,
) -> Option<String> {
    let parts = items
        .iter()
        .map(|item| item.render_inline(options))
        .collect::<Option<Vec<_>>>()?;

    if parts.is_empty() {
        Some(empty.to_string())
    } else {
        Some(format!("{open}{}{close}", parts.join(", ")))
    }
}

fn format_field(field: &Field, indent: usize, add_comma: bool, options: FormatOptions) -> String {
    let prefix = field.prefix();
    let mut out = prefix.clone();
    out.push_str(&format_value(
        &field.value,
        indent,
        indent + text_width(&prefix),
        options,
    ));
    push_trailing_comment(&mut out, field.trailing_comment.as_ref(), add_comma);
    out
}

fn format_field_inline(field: &Field, options: FormatOptions) -> Option<String> {
    if field.trailing_comment.is_some() {
        return None;
    }

    let mut out = field.prefix();
    out.push_str(&format_value_inline(&field.value, options)?);
    Some(out)
}

fn format_include_stmt(include: &IncludeStmt, add_comma: bool) -> String {
    let mut out = include.include.render();
    push_trailing_comment(&mut out, include.trailing_comment.as_ref(), add_comma);
    out
}

fn format_include_stmt_inline(include: &IncludeStmt) -> Option<String> {
    if include.trailing_comment.is_some() {
        return None;
    }

    Some(include.include.render())
}

fn push_trailing_comment(
    out: &mut String,
    trailing_comment: Option<&TrailingComment>,
    add_comma: bool,
) {
    match trailing_comment {
        Some(comment) => {
            if add_comma {
                out.push(',');
            }
            out.push_str(&comment.text);
        }
        None if add_comma => out.push(','),
        None => {}
    }
}

fn push_multiline_item_separator(out: &mut String, index: usize, leading_blank_line: bool) {
    if index > 0 {
        if leading_blank_line {
            out.push_str("\n\n");
        } else {
            out.push('\n');
        }
    }
}

fn should_add_comma(
    is_comma_eligible: bool,
    is_last_comma_eligible: bool,
    comma_style: CommaStyle,
) -> bool {
    if !is_comma_eligible {
        return false;
    }

    match comma_style {
        CommaStyle::None => false,
        CommaStyle::Commas => !is_last_comma_eligible,
        CommaStyle::Trailing => true,
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
            for (index, item) in items.iter().enumerate() {
                if index > 0
                    && should_insert_concat_space(
                        &items[index - 1].part,
                        &item.separator,
                        &item.part,
                    )
                {
                    out.push(' ');
                }
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
            for (index, item) in items.iter().enumerate() {
                if index > 0
                    && should_insert_concat_space(
                        &items[index - 1].part,
                        &item.separator,
                        &item.part,
                    )
                {
                    out.push(' ');
                }
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

fn should_insert_concat_space(previous: &ValuePart, separator: &str, current: &ValuePart) -> bool {
    if matches!(previous, ValuePart::Object(_) | ValuePart::Array(_))
        || matches!(current, ValuePart::Object(_) | ValuePart::Array(_))
    {
        return false;
    }

    if !separator.is_empty() {
        return true;
    }

    !matches!(
        (previous, current),
        (
            ValuePart::Atom(Atom::Number(_)),
            ValuePart::Atom(Atom::Unquoted(_))
        )
    )
}

fn is_safe_unquoted_path_segment(segment: &str) -> bool {
    !segment.is_empty()
        && !segment.contains("//")
        && segment
            .chars()
            .all(|ch| !is_forbidden_unquoted_char(ch, true))
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
    SawCommaThenNewline,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct EntrySeparator {
    had_separator: bool,
    leading_blank_line: bool,
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
        let mut entries = Vec::new();
        let mut leading_blank_line = self.consume_layout_without_comments() > 1;
        loop {
            leading_blank_line = self.collect_standalone_comments(&mut entries, leading_blank_line);

            if terminator.is_none() && self.peek_char() == Some('}') {
                return Err(self.error("unexpected closing '}' in implicit root object"));
            }

            if self.is_object_end(terminator) {
                break;
            }

            let mut entry = self.parse_entry()?;
            entry.set_leading_blank_line(leading_blank_line);
            entries.push(entry);
            if let Some(comment) = self.consume_inline_comment_suffix() {
                entries.last_mut().unwrap().attach_trailing_comment(comment);
            }

            if self.is_object_end(terminator) {
                break;
            }

            let separator = self
                .consume_body_separator(&mut entries, "unexpected comma between object entries")?;
            leading_blank_line = separator.leading_blank_line;
            if terminator.is_none() && self.peek_char() == Some('}') {
                return Err(self.error("unexpected closing '}' in implicit root object"));
            }
            if self.is_object_end(terminator) {
                break;
            }
            if !separator.had_separator {
                return Err(self.error("expected a comma or newline between object entries"));
            }
        }

        if terminator == Some('}') {
            self.expect_char('}')?;
        }

        Ok(ObjectValue {
            entries,
            prefer_multiline: contains_newline(&self.input[content_start..self.pos]),
        })
    }

    fn parse_array_items(&mut self) -> Result<ArrayValue, ParseError> {
        self.expect_char('[')?;
        let content_start = self.pos;
        let mut items = Vec::new();
        let mut leading_blank_line = self.consume_layout_without_comments() > 1;
        loop {
            leading_blank_line = self.collect_standalone_comments(&mut items, leading_blank_line);

            if self.peek_char() == Some(']') {
                break;
            }

            let value = self.parse_value()?;
            items.push(ArrayItem::Value {
                leading_blank_line,
                value,
                trailing_comment: None,
            });
            if let Some(comment) = self.consume_inline_comment_suffix() {
                items.last_mut().unwrap().attach_trailing_comment(comment);
            }

            if self.peek_char() == Some(']') {
                break;
            }

            let separator =
                self.consume_body_separator(&mut items, "unexpected comma between array elements")?;
            leading_blank_line = separator.leading_blank_line;
            if self.peek_char() == Some(']') {
                break;
            }
            if !separator.had_separator {
                return Err(self.error("expected a comma or newline between array elements"));
            }
        }

        self.expect_char(']')?;
        Ok(ArrayValue {
            items,
            prefer_multiline: contains_newline(&self.input[content_start..self.pos]),
        })
    }

    fn parse_entry(&mut self) -> Result<Entry, ParseError> {
        if self.starts_include_statement() {
            Ok(Entry::Include(IncludeStmt {
                leading_blank_line: false,
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
            leading_blank_line: false,
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
            if self.starts_comment() || self.at_newline() || self.is_value_terminator() {
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
            self.validate_concat_items(&items)?;
            Ok(Value::Concat(items))
        }
    }

    fn validate_concat_items(&self, items: &[ConcatItem]) -> Result<(), ParseError> {
        let mut concat_kind = None;

        for item in items {
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

        Ok(())
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
                || self.at_newline()
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
            if self.starts_comment() || is_forbidden_unquoted_char(ch, stop_at_dot) {
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
        self.can_start_scalar_token(false, false)
    }

    fn can_start_path_token(&self) -> bool {
        self.can_start_scalar_token(true, true)
    }

    fn can_start_scalar_token(&self, stop_at_dot: bool, allow_quoted: bool) -> bool {
        (allow_quoted && self.peek_char() == Some('"'))
            || self.match_keyword_atom().is_some()
            || matches!(self.peek_char(), Some(ch) if ch.is_ascii_digit() || ch == '-')
            || self.can_start_unquoted(stop_at_dot)
    }

    fn can_start_unquoted(&self, stop_at_dot: bool) -> bool {
        match self.peek_char() {
            Some(ch) => !is_forbidden_unquoted_char(ch, stop_at_dot) && !self.starts_comment(),
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

    fn consume_body_separator<T: BodyItem>(
        &mut self,
        items: &mut Vec<T>,
        unexpected_comma_message: &str,
    ) -> Result<EntrySeparator, ParseError> {
        let mut state = SeparatorState::Start;
        let mut newline_count = 0;

        loop {
            let ws = self.consume_inline_whitespace();

            if self.peek_char() == Some(',') {
                if matches!(
                    state,
                    SeparatorState::SawComma | SeparatorState::SawCommaThenNewline
                ) {
                    return Err(self.error(unexpected_comma_message));
                }
                self.pos += 1;
                newline_count = 0;
                state = SeparatorState::SawComma;
                continue;
            }

            if self.starts_comment() {
                let comment = self.consume_comment_text();
                if state == SeparatorState::SawComma {
                    items
                        .last_mut()
                        .unwrap()
                        .attach_trailing_comment(TrailingComment {
                            text: format!("{}{}", ws, comment),
                        });
                } else {
                    items.push(T::comment(StandaloneComment {
                        text: comment,
                        leading_blank_line: newline_count > 1,
                    }));
                    newline_count = 0;
                }
                continue;
            }

            if self.consume_newline() {
                newline_count += 1;
                state = match state {
                    SeparatorState::SawComma | SeparatorState::SawCommaThenNewline => {
                        SeparatorState::SawCommaThenNewline
                    }
                    _ => SeparatorState::SawNewline,
                };
                continue;
            }

            break;
        }

        Ok(EntrySeparator {
            had_separator: state != SeparatorState::Start,
            leading_blank_line: newline_count > 1,
        })
    }

    fn collect_standalone_comments<T: BodyItem>(
        &mut self,
        items: &mut Vec<T>,
        mut leading_blank_line: bool,
    ) -> bool {
        loop {
            leading_blank_line |= self.consume_layout_without_comments() > 1;
            if !self.starts_comment() {
                return leading_blank_line;
            }
            items.push(T::comment(StandaloneComment {
                text: self.consume_comment_text(),
                leading_blank_line,
            }));
            leading_blank_line = false;
        }
    }

    fn consume_inline_comment_suffix(&mut self) -> Option<TrailingComment> {
        let before = self.pos;
        let ws = self.consume_inline_whitespace();
        if self.starts_comment() {
            return Some(TrailingComment {
                text: format!("{}{}", ws, self.consume_comment_text()),
            });
        }
        self.pos = before;
        None
    }

    fn consume_ws_or_comment_separator(&mut self) -> bool {
        let before = self.pos;
        let _ = self.consume_inline_whitespace();
        if self.starts_comment() || self.at_newline() {
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

            while self.consume_newline() {
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
            if self.at_newline() {
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
        self.consume_layout_without_comments();
    }

    fn consume_layout_without_comments(&mut self) -> usize {
        let mut newline_count = 0;

        loop {
            let before = self.pos;
            let _ = self.consume_inline_whitespace();
            while self.consume_newline() {
                newline_count += 1;
                let _ = self.consume_inline_whitespace();
            }
            if self.pos == before {
                break;
            }
        }

        newline_count
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

    fn at_newline(&self) -> bool {
        self.newline_len().is_some()
    }

    fn consume_newline(&mut self) -> bool {
        let Some(len) = self.newline_len() else {
            return false;
        };
        self.pos += len;
        true
    }

    fn newline_len(&self) -> Option<usize> {
        if self.starts_with("\r\n") {
            Some(2)
        } else if matches!(self.peek_char(), Some(ch) if is_newline_char(ch)) {
            Some(1)
        } else {
            None
        }
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

fn is_forbidden_unquoted_char(ch: char, stop_at_dot: bool) -> bool {
    is_newline_char(ch)
        || ch.is_whitespace()
        || matches!(
            ch,
            '$' | '"'
                | '{'
                | '}'
                | '['
                | ']'
                | ':'
                | '='
                | ','
                | '#'
                | '`'
                | '^'
                | '?'
                | '!'
                | '@'
                | '*'
                | '&'
                | '\\'
                | '+'
        )
        || (stop_at_dot && ch == '.')
}

fn line_column_at(input: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    let mut chars = input[..offset.min(input.len())].chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            line += 1;
            column = 1;
        } else if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }

    (line, column)
}

fn contains_newline(text: &str) -> bool {
    text.chars().any(is_newline_char)
}

fn is_newline_char(ch: char) -> bool {
    matches!(ch, '\n' | '\r')
}

fn is_inline_whitespace(ch: char) -> bool {
    !is_newline_char(ch) && (ch.is_whitespace() || ch == '\u{feff}')
}
