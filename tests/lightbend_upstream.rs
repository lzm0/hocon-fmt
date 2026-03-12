use std::fs;
use std::path::{Path, PathBuf};

use hocon_fmt::format_hocon;
use indoc::indoc;

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

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lightbend-config")
}

fn fixture_path(relative: &str) -> PathBuf {
    fixtures_root().join(relative)
}

fn read_fixture(relative: &str) -> String {
    fs::read_to_string(fixture_path(relative))
        .unwrap_or_else(|error| panic!("failed to read fixture {relative}: {error}"))
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
            fixture_path(relative).is_file(),
            "missing ported upstream fixture {relative}"
        );
    }
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
        assert_format_idempotent(&read_fixture(relative), relative);
    }
}

#[test]
fn ported_lightbend_reference_fixtures_parse_and_format_stably() {
    for relative in PORTED_REFERENCE_FIXTURES {
        assert_format_idempotent(&read_fixture(relative), relative);
    }
}

#[test]
fn ported_lightbend_include_support_fixtures_parse_and_format_stably() {
    for relative in PORTED_INCLUDE_SUPPORT_FIXTURES {
        assert_format_idempotent(&read_fixture(relative), relative);
    }
}

#[test]
fn ported_from_concatenation_test_string_concat_cannot_span_lines() {
    let input = indoc! {"
        a : ${x}
        foo, x = 1
    "};
    assert!(format_hocon(input).is_err());
}

#[test]
fn ported_from_concatenation_test_list_concat_cannot_span_lines() {
    let input = indoc! {"
        a : [1,2]
        [3,4]
    "};
    assert!(format_hocon(input).is_err());
}

#[test]
fn ported_from_concatenation_test_object_concat_cannot_span_lines() {
    let input = indoc! {"
        a : { b : c }
        { x : y }
    "};
    assert!(format_hocon(input).is_err());
}

#[test]
fn ported_from_concatenation_test_string_concat_inside_array_value() {
    let input = "a : [ foo bar 10 ]";
    let expected = indoc! {"
        a = [
          foo bar 10
        ]
    "};

    assert_eq!(format_hocon(input).unwrap(), expected);
}

#[test]
fn ported_from_concatenation_test_string_concats_are_keys() {
    let input = "123 foo : \"value\"";
    let expected = "\"123 foo\" = \"value\"\n";

    assert_eq!(format_hocon(input).unwrap(), expected);
}

#[test]
fn ported_from_concatenation_test_objects_are_not_keys() {
    let input = "{ { a : 1 } : \"value\" }";
    assert!(format_hocon(input).is_err());
}

#[test]
fn ported_from_concatenation_test_arrays_are_not_keys() {
    let input = "{ [ \"a\" ] : \"value\" }";
    assert!(format_hocon(input).is_err());
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
