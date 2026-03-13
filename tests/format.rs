use std::fs;
use std::path::{Path, PathBuf};

mod support;

use hocon_fmt::{CommaStyle, FormatOptions, format_hocon, format_hocon_with_options};
use support::{read_fixture, read_input_fixture};

const PORTED_EQUIV_CASES: &[&str] = &[
    "equiv01/comments.conf",
    "equiv01/equals.conf",
    "equiv01/no-commas.conf",
    "equiv01/no-root-braces.conf",
    "equiv01/no-whitespace.json",
    "equiv01/omit-colons.conf",
    "equiv01/path-keys.conf",
    "equiv01/properties-style.conf",
    "equiv01/substitutions.conf",
    "equiv01/unquoted.conf",
    "equiv02/path-keys.conf",
    "equiv02/path-keys-weird-whitespace.conf",
    "equiv03/includes.conf",
    "equiv04/missing-substitutions.conf",
    "equiv05/triple-quotes.conf",
];

const PORTED_REFERENCE_FIXTURES: &[&str] = &[
    "equiv01/original.json",
    "equiv02/original.json",
    "equiv03/original.json",
    "equiv04/original.json",
    "equiv05/original.json",
];

const PORTED_INCLUDE_SUPPORT_FIXTURES: &[&str] = &[
    "equiv03/letters/a.conf",
    "equiv03/letters/b.json",
    "equiv03/letters/c.conf",
    "equiv03/letters/c.properties",
    "equiv03/letters/numbers/1.conf",
    "equiv03/letters/numbers/2.properties",
    "equiv03/root/foo.conf",
];

fn fixture_file(case: &str, kind: &str) -> String {
    format!("format/{case}/{kind}.conf")
}

fn lightbend_fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("lightbend-config")
}

fn lightbend_fixture_path(relative: &str) -> PathBuf {
    lightbend_fixtures_root().join(relative)
}

fn read_lightbend_fixture(relative: &str) -> String {
    fs::read_to_string(lightbend_fixture_path(relative))
        .unwrap_or_else(|error| panic!("failed to read fixture {relative}: {error}"))
}

fn assert_formats(case: &str) {
    let input = read_input_fixture(&fixture_file(case, "input"));
    let expected = read_fixture(&fixture_file(case, "expected"));

    assert_eq!(format_hocon(&input).unwrap(), expected);
}

fn assert_formats_with_options(case: &str, options: FormatOptions) {
    let input = read_input_fixture(&fixture_file(case, "input"));
    let expected = read_fixture(&fixture_file(case, "expected"));

    assert_eq!(
        format_hocon_with_options(&input, options).unwrap(),
        expected
    );
}

fn assert_formats_to(input: &str, expected: &str) {
    assert_eq!(format_hocon(input).unwrap(), expected);
}

fn assert_format_idempotent(input: &str, context: &str) {
    let formatted =
        format_hocon(input).unwrap_or_else(|error| panic!("{context} should parse: {error}"));
    let reformatted = format_hocon(&formatted)
        .unwrap_or_else(|error| panic!("{context} should reparse after formatting: {error}"));
    assert_eq!(
        reformatted, formatted,
        "{context} should be stable after formatting"
    );
}

fn assert_fixture_set_exists(fixtures: &[&str]) {
    for relative in fixtures {
        assert!(
            lightbend_fixture_path(relative).is_file(),
            "missing ported upstream fixture {relative}"
        );
    }
}

#[test]
fn formats_implicit_root_object_and_nested_values() {
    assert_formats("default/implicit_root_object_and_nested_values");
}

#[test]
fn preserves_literal_concatenation_spacing() {
    assert_formats("default/preserves_literal_concatenation_spacing");
}

#[test]
fn formats_includes_substitutions_and_append() {
    assert_formats("default/includes_substitutions_and_append");
}

#[test]
fn formats_object_and_array_concatenation() {
    assert_formats("default/object_and_array_concatenation");
}

#[test]
fn supports_concatenation_inside_arrays() {
    assert_formats("default/concatenation_inside_arrays");
}

#[test]
fn formats_path_segments_with_spaces_canonically() {
    assert_formats("default/path_segments_with_spaces");
}

#[test]
fn supports_environment_list_substitutions() {
    assert_formats("default/environment_list_substitutions");
}

#[test]
fn accepts_numbers_followed_by_unquoted_concatenation() {
    assert_formats("default/numbers_followed_by_unquoted_concatenation");
}

#[test]
fn preserves_explicit_root_object() {
    assert_formats("default/explicit_root_object");
}

#[test]
fn accepts_numeric_path_components() {
    assert_formats("default/numeric_path_components");
}

#[test]
fn formats_with_commas_between_elements() {
    assert_formats_with_options(
        "options/commas_between_elements",
        FormatOptions {
            comma_style: CommaStyle::Commas,
            max_width: 1,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn formats_with_trailing_commas() {
    assert_formats_with_options(
        "options/trailing_commas",
        FormatOptions {
            comma_style: CommaStyle::Trailing,
            max_width: 1,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn does_not_add_commas_to_implicit_root_entries() {
    assert_formats_with_options(
        "options/no_implicit_root_commas",
        FormatOptions {
            comma_style: CommaStyle::Trailing,
            max_width: 1,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn limits_root_separation_to_one_blank_line() {
    assert_formats("default/root_separation");
}

#[test]
fn does_not_insert_blank_lines_between_root_includes() {
    assert_formats("default/root_includes_no_blank_line");
}

#[test]
fn preserves_root_level_comments() {
    assert_formats("default/root_level_comments");
}

#[test]
fn preserves_comments_inside_objects_and_arrays() {
    assert_formats("default/comments_inside_objects_and_arrays");
}

#[test]
fn preserves_inline_comments_on_same_line() {
    assert_formats("default/inline_comments");
}

#[test]
fn preserves_inline_comments_after_commas() {
    assert_formats("default/inline_comments_after_commas");
}

#[test]
fn preserves_newline_comments_after_commas() {
    assert_formats("default/newline_comments_after_commas");
}

#[test]
fn places_commas_before_inline_comments_when_enabled() {
    assert_formats_with_options(
        "options/commas_before_inline_comments",
        FormatOptions {
            comma_style: CommaStyle::Commas,
            max_width: 80,
        },
    );
}

#[test]
fn keeps_newline_comments_standalone_when_commas_enabled() {
    assert_formats_with_options(
        "options/newline_comments_standalone_with_commas",
        FormatOptions {
            comma_style: CommaStyle::Commas,
            max_width: 80,
        },
    );
}

#[test]
fn ported_lightbend_fixture_inventory_is_present() {
    assert_eq!(PORTED_EQUIV_CASES.len(), 15);
    assert_eq!(PORTED_REFERENCE_FIXTURES.len(), 5);
    assert_eq!(PORTED_INCLUDE_SUPPORT_FIXTURES.len(), 7);

    assert_fixture_set_exists(PORTED_EQUIV_CASES);
    assert_fixture_set_exists(PORTED_REFERENCE_FIXTURES);
    assert_fixture_set_exists(PORTED_INCLUDE_SUPPORT_FIXTURES);
}

#[test]
fn ported_lightbend_equiv_cases_parse_and_format_stably() {
    for relative in PORTED_EQUIV_CASES {
        assert_format_idempotent(&read_lightbend_fixture(relative), relative);
    }
}

#[test]
fn ported_lightbend_reference_fixtures_parse_and_format_stably() {
    for relative in PORTED_REFERENCE_FIXTURES {
        assert_format_idempotent(&read_lightbend_fixture(relative), relative);
    }
}

#[test]
fn ported_lightbend_include_support_fixtures_parse_and_format_stably() {
    for relative in PORTED_INCLUDE_SUPPORT_FIXTURES {
        assert_format_idempotent(&read_lightbend_fixture(relative), relative);
    }
}

#[test]
fn ported_from_concatenation_test_string_concat_cannot_span_lines() {
    let input = "a : ${x}\nfoo, x = 1";
    assert!(format_hocon(&input).is_err());
}

#[test]
fn ported_from_concatenation_test_list_concat_cannot_span_lines() {
    let input = "a : [1,2]\n[3,4]";
    assert!(format_hocon(&input).is_err());
}

#[test]
fn ported_from_concatenation_test_object_concat_cannot_span_lines() {
    let input = "a : { b : c }\n{ x : y }";
    assert!(format_hocon(&input).is_err());
}

#[test]
fn ported_from_concatenation_test_string_concat_inside_array_value() {
    assert_formats("concatenation/string_concat_inside_array_value");
}

#[test]
fn ported_from_concatenation_test_string_concats_are_keys() {
    assert_formats("concatenation/string_concats_are_keys");
}

#[test]
fn ported_from_concatenation_test_objects_are_not_keys() {
    let input = "{ { a : 1 } : \"value\" }";
    assert!(format_hocon(&input).is_err());
}

#[test]
fn ported_from_concatenation_test_arrays_are_not_keys() {
    let input = "{ [ \"a\" ] : \"value\" }";
    assert!(format_hocon(&input).is_err());
}

#[test]
fn ported_from_tokenizer_test_invalid_strings_are_rejected() {
    for input in [
        "\"\\q\"",
        "\"\\u123\"",
        "\"\\u12\"",
        "\"\\u1\"",
        "\"\\u\"",
        "\"",
        "\"abcdefg",
        "$",
        "${",
    ] {
        assert!(
            format_hocon(input).is_err(),
            "expected parse failure for {input:?}"
        );
    }
}

#[test]
fn accepts_empty_implicit_root_documents() {
    for input in ["", " \n\t\u{feff}\n"] {
        assert_eq!(
            format_hocon(input).unwrap(),
            "",
            "expected empty output for {input:?}"
        );
    }
}

#[test]
fn accepts_comment_only_implicit_root_documents() {
    for input in ["# comment only\n", "// comment only\n"] {
        assert_eq!(
            format_hocon(input).unwrap(),
            input,
            "expected comments to round-trip for {input:?}"
        );
    }
}

#[test]
fn rejects_unmatched_closing_braces_in_implicit_root_documents() {
    for input in ["}", "a = 1}", "a = 1\n}", "a = { b = 1 }\n}"] {
        assert!(
            format_hocon(input).is_err(),
            "expected parse failure for {input:?}"
        );
    }
}

#[test]
fn rejects_repeated_or_misplaced_commas() {
    for input in [
        "[1,2,3,,]",
        "[,1,2,3]",
        "[1,,2,3]",
        "[1\n,2]",
        "{ a : 1,, }",
        "{ , a : 1 }",
        "{ a : 1\n, b : 2 }",
    ] {
        assert!(
            format_hocon(input).is_err(),
            "expected parse failure for {input:?}"
        );
    }
}

#[test]
fn rejects_mixed_type_value_concatenation() {
    for input in [
        "a = { b : 1 } 2",
        "a = [1] 2",
        "a = [1] { b : 2 }",
        "a = { b : 1 } [2]",
    ] {
        assert!(
            format_hocon(input).is_err(),
            "expected parse failure for {input:?}"
        );
    }
}

#[test]
fn rejects_raw_control_characters_in_quoted_strings() {
    for input in ["a = \"\t\"\n", "a = \"\r\"\n", "a = \"\u{1}\"\n"] {
        assert!(
            format_hocon(input).is_err(),
            "expected parse failure for {input:?}"
        );
    }
}

#[test]
fn formats_include_and_path_edge_cases_canonically() {
    assert_formats_to("{ foo include : 42 }", "{ \"foo include\" = 42 }\n");
    assert_formats_to("{ \"include\" : 42 }", "{ \"include\" = 42 }\n");
    assert_formats_to(
        "include\n required ( file ( \"x.conf\" ) )",
        "include required(file(\"x.conf\"))\n",
    );
    assert_formats_to("a.\"\".b = 1", "a.\"\".b = 1\n");
}

#[test]
fn keeps_single_line_collections_when_they_fit() {
    assert_formats_to("a:{b=1,c:[2,3]}", "a = { b = 1, c = [2, 3] }\n");
}

#[test]
fn breaks_collections_when_they_exceed_max_width() {
    let output = format_hocon_with_options(
        "a:{b=1,c:[2,3]}",
        FormatOptions {
            max_width: 10,
            ..FormatOptions::default()
        },
    )
    .unwrap();

    assert_eq!(output, "a = {\n  b = 1\n  c = [\n    2\n    3\n  ]\n}\n");
}

#[test]
fn preserves_multiline_arrays_even_when_they_would_fit() {
    assert_formats_to("a = [\n  1, 2\n]\n", "a = [\n  1\n  2\n]\n");
}

#[test]
fn rejects_empty_path_elements() {
    for input in ["a..b = 1", ".a = 1", "a. = 1", "a = ${foo..bar}"] {
        assert!(
            format_hocon(input).is_err(),
            "expected parse failure for {input:?}"
        );
    }
}
