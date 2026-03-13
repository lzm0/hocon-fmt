use std::fs;
use std::path::{Path, PathBuf};

pub fn fixture_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative)
}

pub fn read_fixture(relative: &str) -> String {
    fs::read_to_string(fixture_path(relative))
        .unwrap_or_else(|error| panic!("failed to read fixture {relative}: {error}"))
}

pub fn read_input_fixture(relative: &str) -> String {
    let mut content = read_fixture(relative);

    if content.ends_with('\n') {
        content.pop();

        if content.ends_with('\r') {
            content.pop();
        }
    }

    content
}
