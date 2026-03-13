mod support;

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use support::{read_fixture, read_input_fixture};

fn fixture_file(case: &str, kind: &str) -> String {
    format!("cli/{case}/{kind}.conf")
}

fn binary_path() -> PathBuf {
    std::env::var_os("CARGO_BIN_EXE_hocon-fmt")
        .or_else(|| std::env::var_os("CARGO_BIN_EXE_hocon_fmt"))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("target")
                .join("debug")
                .join(format!("hocon-fmt{}", std::env::consts::EXE_SUFFIX))
        })
}

fn run_cli(args: &[&str], stdin: Option<&str>) -> Output {
    let mut command = Command::new(binary_path());
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if stdin.is_some() {
        command.stdin(Stdio::piped());
    }

    let mut child = command.spawn().expect("failed to spawn CLI");

    if let Some(input) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin pipe missing")
            .write_all(input.as_bytes())
            .expect("failed to write stdin");
    }

    child.wait_with_output().expect("failed to wait on CLI")
}

fn unique_temp_dir() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock drift")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("hocon-fmt-cli-{}-{}", std::process::id(), nonce));
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}

#[test]
fn formats_stdin_to_stdout_when_no_file_is_given() {
    let input = read_input_fixture(&fixture_file("single_field_object", "input"));
    let output = run_cli(&[], Some(&input));

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        read_fixture(&fixture_file("single_field_object", "expected"))
    );
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
}

#[test]
fn formats_with_no_commas_when_requested() {
    let input = read_input_fixture(&fixture_file("compact_nested_object", "input"));
    let output = run_cli(&["--commas", "none", "--max-width", "1"], Some(&input));

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        read_fixture(&fixture_file(
            "formats_with_no_commas_when_requested",
            "expected",
        ))
    );
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
}

#[test]
fn formats_with_standard_commas_when_requested() {
    let input = read_input_fixture(&fixture_file("compact_nested_object", "input"));
    let output = run_cli(&["--commas", "commas", "--max-width", "1"], Some(&input));

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        read_fixture(&fixture_file(
            "formats_with_standard_commas_when_requested",
            "expected",
        ))
    );
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
}

#[test]
fn formats_with_trailing_commas_when_requested() {
    let input = read_input_fixture(&fixture_file(
        "formats_with_trailing_commas_when_requested",
        "input",
    ));
    let output = run_cli(&["--commas", "trailing", "--max-width", "1"], Some(&input));

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        read_fixture(&fixture_file(
            "formats_with_trailing_commas_when_requested",
            "expected",
        ))
    );
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
}

#[test]
fn check_mode_reports_unformatted_files() {
    let dir = unique_temp_dir();
    let file = dir.join("input.conf");
    fs::write(&file, "a:{b=1}").unwrap();

    let output = run_cli(&["--check", file.to_str().unwrap()], None);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stdout).unwrap().is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("would reformat"));
    assert!(stderr.contains("input.conf"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn write_mode_formats_files_in_place() {
    let dir = unique_temp_dir();
    let file = dir.join("input.conf");
    fs::write(
        &file,
        read_input_fixture(&fixture_file("single_field_object", "input")),
    )
    .unwrap();

    let output = run_cli(&["--write", file.to_str().unwrap()], None);

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        read_fixture(&fixture_file("single_field_object", "expected"))
    );
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("formatted")
    );

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn output_mode_writes_to_a_different_file() {
    let dir = unique_temp_dir();
    let input = dir.join("input.conf");
    let output_path = dir.join("output.conf");
    let original = read_input_fixture(&fixture_file("single_field_object", "input"));
    fs::write(&input, &original).unwrap();

    let output = run_cli(
        &[
            "--output",
            output_path.to_str().unwrap(),
            input.to_str().unwrap(),
        ],
        None,
    );

    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout).unwrap().is_empty());
    assert_eq!(
        fs::read_to_string(&output_path).unwrap(),
        read_fixture(&fixture_file("single_field_object", "expected"))
    );
    assert_eq!(fs::read_to_string(&input).unwrap(), original);

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn multiple_inputs_require_a_multi_file_mode() {
    let dir = unique_temp_dir();
    let first = dir.join("first.conf");
    let second = dir.join("second.conf");
    fs::write(&first, "a:{b=1}").unwrap();
    fs::write(&second, "c:{d=2}").unwrap();

    let output = run_cli(&[first.to_str().unwrap(), second.to_str().unwrap()], None);

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("multiple input files require --check or --write"));

    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn formats_inline_collections_by_default_when_they_fit() {
    let output = run_cli(&["--max-width", "80"], Some("a:{b=1,c:[2,3]}"));

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "a = { b = 1, c = [ 2, 3 ] }\n"
    );
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
}

#[test]
fn formats_multiline_when_max_width_is_narrow() {
    let output = run_cli(&["--max-width", "10"], Some("a:{b=1,c:[2,3]}"));

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "a = {\n  b = 1\n  c = [\n    2\n    3\n  ]\n}\n"
    );
    assert!(String::from_utf8(output.stderr).unwrap().is_empty());
}

#[test]
fn rejects_zero_max_width() {
    let output = run_cli(&["--max-width", "0"], Some("a:{b=1}"));

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--max-width must be greater than zero"));
}
