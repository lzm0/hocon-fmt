use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "hocon.pest"]
pub struct HoconParser;

fn main() {
    let file_path = std::env::args().nth(1).expect("File path missing");
    let content = std::fs::read_to_string(file_path).unwrap();

    let parse_result = HoconParser::parse(Rule::hocon, &content);
    match parse_result {
        Ok(pairs) => {
            println!("{:?}", pairs);
        }
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    }
}
