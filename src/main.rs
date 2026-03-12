use hocon_formatter::format_hocon;

fn main() {
    let file_path = std::env::args().nth(1).expect("file path missing");
    let input = std::fs::read_to_string(file_path).expect("failed to read input file");

    match format_hocon(&input) {
        Ok(formatted) => print!("{formatted}"),
        Err(error) => {
            eprintln!("Parsing error: {error}");
            std::process::exit(1);
        }
    }
}
