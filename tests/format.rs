mod support;

use hocon_fmt::{CommaStyle, FormatOptions, format_hocon, format_hocon_with_options};
use support::{read_fixture, read_input_fixture};

const LIGHTBEND_FORMAT_CASES: &[&str] = &[
    "lightbend_equiv01_comments_conf",
    "lightbend_equiv01_equals_conf",
    "lightbend_equiv01_no_commas_conf",
    "lightbend_equiv01_no_root_braces_conf",
    "lightbend_equiv01_no_whitespace_json",
    "lightbend_equiv01_omit_colons_conf",
    "lightbend_equiv01_original_json",
    "lightbend_equiv01_path_keys_conf",
    "lightbend_equiv01_properties_style_conf",
    "lightbend_equiv01_substitutions_conf",
    "lightbend_equiv01_unquoted_conf",
    "lightbend_equiv02_original_json",
    "lightbend_equiv02_path_keys_conf",
    "lightbend_equiv02_path_keys_weird_whitespace_conf",
    "lightbend_equiv03_includes_conf",
    "lightbend_equiv03_letters_a_conf",
    "lightbend_equiv03_letters_b_json",
    "lightbend_equiv03_letters_c_conf",
    "lightbend_equiv03_letters_c_properties",
    "lightbend_equiv03_letters_numbers_1_conf",
    "lightbend_equiv03_letters_numbers_2_properties",
    "lightbend_equiv03_original_json",
    "lightbend_equiv03_root_foo_conf",
    "lightbend_equiv04_missing_substitutions_conf",
    "lightbend_equiv04_original_json",
    "lightbend_equiv05_original_json",
    "lightbend_equiv05_triple_quotes_conf",
];

fn fixture_file(case: &str, kind: &str) -> String {
    format!("format/{case}/{kind}.conf")
}

fn assert_formats(case: &str) {
    let input = read_input_fixture(&fixture_file(case, "input"));
    let expected = read_fixture(&fixture_file(case, "expected"));

    assert_eq!(format_hocon(&input).unwrap(), expected);
    assert_eq!(format_hocon(&expected).unwrap(), expected);
}

fn assert_formats_with_options(case: &str, options: FormatOptions) {
    let input = read_input_fixture(&fixture_file(case, "input"));
    let expected = read_fixture(&fixture_file(case, "expected"));

    assert_eq!(
        format_hocon_with_options(&input, options).unwrap(),
        expected
    );
    assert_eq!(
        format_hocon_with_options(&expected, options).unwrap(),
        expected
    );
}

fn assert_formats_to(input: &str, expected: &str) {
    assert_eq!(format_hocon(input).unwrap(), expected);
    assert_eq!(format_hocon(expected).unwrap(), expected);
}

#[test]
fn formats_implicit_root_object_and_nested_values() {
    assert_formats("implicit_root_object_and_nested_values");
}

#[test]
fn preserves_literal_concatenation_spacing() {
    assert_formats("preserves_literal_concatenation_spacing");
}

#[test]
fn formats_includes_substitutions_and_append() {
    assert_formats("includes_substitutions_and_append");
}

#[test]
fn formats_object_and_array_concatenation() {
    assert_formats("object_and_array_concatenation");
}

#[test]
fn supports_concatenation_inside_arrays() {
    assert_formats("concatenation_inside_arrays");
}

#[test]
fn formats_path_segments_with_spaces_canonically() {
    assert_formats("path_segments_with_spaces");
}

#[test]
fn supports_environment_list_substitutions() {
    assert_formats("environment_list_substitutions");
}

#[test]
fn accepts_numbers_followed_by_unquoted_concatenation() {
    assert_formats("numbers_followed_by_unquoted_concatenation");
}

#[test]
fn preserves_explicit_root_object() {
    assert_formats("explicit_root_object");
}

#[test]
fn accepts_numeric_path_components() {
    assert_formats("numeric_path_components");
}

#[test]
fn formats_with_commas_between_elements() {
    assert_formats_with_options(
        "commas_between_elements",
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
        "trailing_commas",
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
        "no_implicit_root_commas",
        FormatOptions {
            comma_style: CommaStyle::Trailing,
            max_width: 1,
            ..FormatOptions::default()
        },
    );
}

#[test]
fn limits_root_separation_to_one_blank_line() {
    assert_formats("root_separation");
}

#[test]
fn does_not_insert_blank_lines_between_root_includes() {
    assert_formats("root_includes_no_blank_line");
}

#[test]
fn preserves_root_level_comments() {
    assert_formats("root_level_comments");
}

#[test]
fn preserves_blank_lines_before_root_level_comments() {
    assert_formats("root_level_blank_line_before_comment");
}

#[test]
fn preserves_comments_inside_objects_and_arrays() {
    assert_formats("comments_inside_objects_and_arrays");
}

#[test]
fn preserves_inline_comments_on_same_line() {
    assert_formats("inline_comments");
}

#[test]
fn preserves_inline_comments_after_commas() {
    assert_formats("inline_comments_after_commas");
}

#[test]
fn preserves_newline_comments_after_commas() {
    assert_formats("newline_comments_after_commas");
}

#[test]
fn accepts_commas_on_their_own_line_between_elements() {
    assert_formats("commas_on_their_own_line");
}

#[test]
fn places_commas_before_inline_comments_when_enabled() {
    assert_formats_with_options(
        "commas_before_inline_comments",
        FormatOptions {
            comma_style: CommaStyle::Commas,
            max_width: 80,
        },
    );
}

#[test]
fn keeps_newline_comments_standalone_when_commas_enabled() {
    assert_formats_with_options(
        "newline_comments_standalone_with_commas",
        FormatOptions {
            comma_style: CommaStyle::Commas,
            max_width: 80,
        },
    );
}

#[test]
fn ported_lightbend_cases_match_snapshots() {
    assert_eq!(LIGHTBEND_FORMAT_CASES.len(), 27);

    for case in LIGHTBEND_FORMAT_CASES {
        assert_formats(case);
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
    assert_formats("string_concat_inside_array_value");
}

#[test]
fn ported_from_concatenation_test_string_concats_are_keys() {
    assert_formats("string_concats_are_keys");
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
        "[1,\n,2]",
        "{ a : 1,, }",
        "{ , a : 1 }",
        "{ a : 1,\n, b : 2 }",
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
