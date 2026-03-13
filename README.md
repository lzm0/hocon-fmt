# hoconfmt

A HOCON formatter written in Rust.

This project parses and formats [HOCON](https://github.com/lightbend/config/blob/main/HOCON.md) with a focus on practical CLI usage and compatibility with the upstream `lightbend/config` test suite.

## Features

- Formats HOCON from stdin or files
- Supports in-place writes with `--write`
- Supports CI-style verification with `--check`
- Supports writing to a separate output file with `--output`
- Supports configurable comma style with `--commas none|commas|trailing`
- Supports width-aware single-line collections with `--max-width`
- Includes ported upstream fixture tests from `lightbend/config`

## Build

```bash
cargo build
```

Run the formatter directly with Cargo:

```bash
cargo run -- path/to/application.conf
```

Or install it locally:

```bash
cargo install --path .
```

## CLI

By default, `hoconfmt` reads a single input and writes the formatted result to stdout.

### Read from stdin

```bash
cat application.conf | hoconfmt
```

### Format a file to stdout

```bash
hoconfmt application.conf
```

### Write changes in place

```bash
hoconfmt --write application.conf
```

### Format all `.conf` files in the current working directory

```bash
find . -type f -name '*.conf' -exec hoconfmt --write {} +
```

### Check formatting without modifying files

```bash
hoconfmt --check application.conf
```

If a file would be reformatted, the command exits with status code `1`.

### Write to a different file

```bash
hoconfmt --output formatted.conf application.conf
```

### Comma style

The formatter supports three separator styles for objects and arrays:

- `--commas none`
  Uses newline separation only
- `--commas commas`
  Uses commas between elements, but not after the last element
- `--commas trailing`
  Uses commas between elements and after the last element

Examples:

```bash
hoconfmt --commas none application.conf
hoconfmt --commas commas application.conf
hoconfmt --commas trailing application.conf
```

### Max width

The formatter keeps arrays and braced objects on one line when they fit within
`--max-width` columns. The default width is `80`.

Arrays and braced objects that are already written across multiple lines stay
multiline, except empty collections, which collapse to `[]` or `{}`.

Examples:

```bash
hoconfmt --max-width 80 application.conf
hoconfmt --max-width 40 application.conf
```

## Library Usage

```rust
use hoconfmt::{format_hocon, format_hocon_with_options, CommaStyle, FormatOptions};

let formatted = format_hocon("a:{b=1}")?;

let formatted_with_commas = format_hocon_with_options(
    "a:{b=1,c:[2,3]}",
    FormatOptions {
        comma_style: CommaStyle::Commas,
        max_width: 40,
    },
)?;
```

## Testing

Run the full test suite with:

```bash
cargo test
```

The test suite includes:

- Unit tests for parser and formatter behavior
- End-to-end CLI tests
- Ported upstream fixtures from `lightbend/config`

## Upstream Test Fixtures

This repository includes snapshot fixtures ported from `lightbend/config` under [`tests/fixtures/format`](./tests/fixtures/format).

## License

This project is licensed under Apache 2.0. See [LICENSE-2.0.txt](./LICENSE-2.0.txt).
