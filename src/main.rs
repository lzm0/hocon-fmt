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
            pairs.for_each(|pair| format_pair(pair));
        }
        Err(e) => {
            println!("{}", e);
            std::process::exit(1);
        }
    }
}

fn format_pair(pair: pest::iterators::Pair<Rule>) {
    for inner_pair in pair.into_inner() {
        match inner_pair.as_rule() {
            _ => {
                println!("{:?}", inner_pair.as_str());
                format_pair(inner_pair);
            }
        }
    }
}
