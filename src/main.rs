use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser, ValueEnum};
use hocon_fmt::{CommaStyle, FormatOptions, format_hocon_with_options};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Format HOCON files",
    long_about = "Format HOCON files from stdin or disk. By default, the formatter reads a single input and writes the result to stdout."
)]
struct Cli {
    #[arg(
        value_name = "FILE",
        help = "Input file(s). Reads stdin when omitted or when '-' is used."
    )]
    inputs: Vec<PathBuf>,

    #[arg(
        short = 'w',
        long = "write",
        visible_alias = "in-place",
        help = "Write the formatted result back to each input file."
    )]
    write: bool,

    #[arg(long, help = "Check whether the input is already formatted.")]
    check: bool,

    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "Write the formatted output to a file instead of stdout."
    )]
    output: Option<PathBuf>,

    #[arg(
        long = "commas",
        value_enum,
        default_value_t = CliCommaStyle::None,
        help = "Comma policy for objects and arrays."
    )]
    comma_style: CliCommaStyle,

    #[arg(
        long = "max-width",
        default_value_t = 80,
        value_name = "COLUMNS",
        help = "Maximum width for keeping collections on one line."
    )]
    max_width: usize,
}

#[derive(Debug, Clone)]
enum InputSource {
    Stdin,
    File(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunOutcome {
    Success,
    CheckFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
enum CliCommaStyle {
    #[default]
    None,
    Commas,
    Trailing,
}

fn main() {
    let cli = Cli::parse();
    validate_cli_or_exit(&cli);

    match run(cli) {
        Ok(RunOutcome::Success) => {}
        Ok(RunOutcome::CheckFailed) => std::process::exit(1),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}

fn validate_cli_or_exit(cli: &Cli) {
    if let Err(message) = validate_cli(cli) {
        Cli::command()
            .error(clap::error::ErrorKind::ValueValidation, message)
            .exit();
    }
}

fn validate_cli(cli: &Cli) -> Result<(), String> {
    let inputs = resolve_inputs(&cli.inputs);
    let stdin_count = inputs
        .iter()
        .filter(|input| matches!(input, InputSource::Stdin))
        .count();

    if cli.write && cli.check {
        return Err("--write and --check cannot be used together".to_string());
    }
    if cli.write && cli.output.is_some() {
        return Err("--write and --output cannot be used together".to_string());
    }
    if cli.check && cli.output.is_some() {
        return Err("--check and --output cannot be used together".to_string());
    }
    if stdin_count > 1 {
        return Err("stdin may only be specified once".to_string());
    }
    if stdin_count == 1 && inputs.len() > 1 {
        return Err("'-' cannot be mixed with file paths".to_string());
    }
    if cli.write && inputs.is_empty() {
        return Err("--write requires at least one input file".to_string());
    }
    if cli.write
        && inputs
            .iter()
            .any(|input| matches!(input, InputSource::Stdin))
    {
        return Err("--write only supports file inputs".to_string());
    }
    if cli.output.is_some() && inputs.len() > 1 {
        return Err("--output only supports a single input".to_string());
    }
    if !cli.write && !cli.check && cli.output.is_none() && inputs.len() > 1 {
        return Err("multiple input files require --check or --write".to_string());
    }
    if cli.max_width == 0 {
        return Err("--max-width must be greater than zero".to_string());
    }

    Ok(())
}

fn run(cli: Cli) -> Result<RunOutcome, String> {
    let options = FormatOptions {
        comma_style: cli.comma_style.into(),
        max_width: cli.max_width,
    };
    let inputs = resolve_inputs(&cli.inputs);

    if cli.write {
        return write_in_place(&inputs, options);
    }
    if cli.check {
        return check_inputs(&inputs, options);
    }

    let source = inputs.first().cloned().unwrap_or(InputSource::Stdin);
    let (_, formatted) = format_source(&source, options)?;

    if let Some(output_path) = cli.output {
        fs::write(&output_path, formatted)
            .map_err(|error| format!("failed to write {}: {error}", output_path.display()))?;
    } else {
        io::stdout()
            .write_all(formatted.as_bytes())
            .map_err(|error| format!("failed to write stdout: {error}"))?;
    }

    Ok(RunOutcome::Success)
}

fn resolve_inputs(paths: &[PathBuf]) -> Vec<InputSource> {
    if paths.is_empty() {
        return vec![InputSource::Stdin];
    }

    paths
        .iter()
        .map(|path| {
            if path == Path::new("-") {
                InputSource::Stdin
            } else {
                InputSource::File(path.clone())
            }
        })
        .collect()
}

fn check_inputs(inputs: &[InputSource], options: FormatOptions) -> Result<RunOutcome, String> {
    let mut needs_formatting = false;

    for source in inputs {
        let (input, formatted) = format_source(source, options)?;
        if formatted != input {
            needs_formatting = true;
            eprintln!("would reformat {}", source.display_name());
        }
    }

    if needs_formatting {
        Ok(RunOutcome::CheckFailed)
    } else {
        Ok(RunOutcome::Success)
    }
}

fn write_in_place(inputs: &[InputSource], options: FormatOptions) -> Result<RunOutcome, String> {
    for source in inputs {
        let InputSource::File(path) = source else {
            return Err("--write only supports file inputs".to_string());
        };

        let (input, formatted) = format_source(source, options)?;
        if formatted != input {
            fs::write(path, formatted)
                .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
            eprintln!("formatted {}", path.display());
        }
    }

    Ok(RunOutcome::Success)
}

fn format_source(source: &InputSource, options: FormatOptions) -> Result<(String, String), String> {
    let input = source.read()?;
    let formatted = format_hocon_with_options(&input, options)
        .map_err(|error| format!("failed to parse {}: {error}", source.display_name()))?;
    Ok((input, formatted))
}

impl From<CliCommaStyle> for CommaStyle {
    fn from(value: CliCommaStyle) -> Self {
        match value {
            CliCommaStyle::None => CommaStyle::None,
            CliCommaStyle::Commas => CommaStyle::Commas,
            CliCommaStyle::Trailing => CommaStyle::Trailing,
        }
    }
}

impl InputSource {
    fn display_name(&self) -> String {
        match self {
            InputSource::Stdin => "<stdin>".to_string(),
            InputSource::File(path) => path.display().to_string(),
        }
    }

    fn read(&self) -> Result<String, String> {
        match self {
            InputSource::Stdin => {
                let mut input = String::new();
                io::stdin()
                    .read_to_string(&mut input)
                    .map_err(|error| format!("failed to read stdin: {error}"))?;
                Ok(input)
            }
            InputSource::File(path) => fs::read_to_string(path)
                .map_err(|error| format!("failed to read {}: {error}", path.display())),
        }
    }
}
