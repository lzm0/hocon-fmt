use std::io::Read;

use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "hocon.pest"]
pub struct HoconParser;

pub fn format_hocon(input: &str) -> Result<String, pest::error::Error<Rule>> {
    let parsed = HoconParser::parse(Rule::hocon, input)?;
    let mut formatted = String::new();

    for pair in parsed {
        format_pair(pair, &mut formatted, 0);
    }
    Ok(formatted)
}

fn format_pair(pair: pest::iterators::Pair<Rule>, output: &mut String, indent: usize) {
    match pair.as_rule() {
        Rule::hocon => {
            for inner_pair in pair.into_inner() {
                format_pair(inner_pair, output, indent);
            }
        }
        Rule::object => {
            output.push_str("{\n");
            for inner_pair in pair.into_inner() {
                format_pair(inner_pair, output, indent + 2);
            }
            output.push_str(&" ".repeat(indent));
            output.push_str("}\n");
        }
        Rule::array => {
            output.push_str("[\n");
            for inner_pair in pair.into_inner() {
                format_pair(inner_pair, output, indent + 2);
            }
            output.push_str(&" ".repeat(indent));
            output.push_str("]\n");
        }
        Rule::field => {
            let mut inner_rules = pair.into_inner();
            let key = inner_rules.next().unwrap().as_str();
            let separator = inner_rules.next().unwrap().as_str().trim();
            output.push_str(&" ".repeat(indent));
            output.push_str(key);
            output.push_str(&format!(" {} ", separator));
            format_pair(inner_rules.next().unwrap(), output, indent);
        }
        Rule::value => {
            for inner_pair in pair.into_inner() {
                format_pair(inner_pair, output, indent);
            }
        }
        Rule::string | Rule::number | Rule::boolean | Rule::null | Rule::value_unquoted_string => {
            output.push_str(pair.as_str());
            output.push('\n');
        }
        Rule::object_body
        | Rule::array_body
        | Rule::object_entry
        | Rule::array_element
        | Rule::root_content => {
            for inner_pair in pair.into_inner() {
                format_pair(inner_pair, output, indent);
            }
        }
        Rule::substitution => {
            output.push_str(pair.as_str());
            output.push('\n');
        }
        Rule::include => {
            output.push_str(&" ".repeat(indent));
            output.push_str(pair.as_str());
            output.push('\n');
        }
        Rule::EOI => {}
        _ => eprintln!("Unexpected rule: {:?}", pair.as_rule()),
    }
}

fn main() {
    let mut input = String::new();
    match std::env::args().nth(1) {
        Some(file_path) => {
            input = std::fs::read_to_string(file_path).unwrap();
        }
        None => {
            std::io::stdin().read_to_string(&mut input).unwrap();
        }
    }
    match format_hocon(&input) {
        Ok(formatted) => println!("{}", formatted),
        Err(e) => eprintln!("Parsing error: {}", e),
    }
}
