use std::str::FromStr;
use winnow::ascii::{alpha1, digit0, space0};
use winnow::combinator::{delimited, opt, repeat, separated, terminated};
use winnow::error::ContextError;
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::token::one_of;
use winnow::{Parser, Result};
// url store.bin <modifiers> { count * [literals or generators(args)] }

// s3:products:/policy_13/
// s3:products:policy_13
// mssql:outcomesidentification:tipresult_new

// s3/products/policy_13/
// s3.products.policy_13
// mssql.outcomesidentification.tipresult_new
#[derive(Debug)]
pub struct Seed;

#[derive(Debug)]
pub struct Instruction {
    pub store_name: String,
}

pub fn prep_recipe(recipe: &str) -> Vec<Instruction> {
    vec![Instruction {
        store_name: String::from(recipe),
    }]
}

trait Recipe {}
impl Recipe for &str {}

#[derive(Debug)]
pub struct Order {
    pub location: Vec<String>,
}

fn parse_stock_items<'i>(input: &mut &'i str) -> Result<(u32, &'i str)> {
    let stock = delimited((space0, "{", space0), "i", (space0, "}", space0));

    let item_count = terminated(digit0, (space0, "*", space0)).map(|s| u32::from_str(s).unwrap());

    let stock_item_parser = (item_count, stock);

    let t = delimited(
        (space0, "[", space0),
        stock_item_parser,
        (space0, "]", space0),
    )
    .parse_next(input);

    println!("{t:#?}");
    t
}

fn parse_order_location<'i>(input: &mut &'i str) -> Result<Vec<String>> {
    separated(0.., alpha1, one_of(['/', '.']))
        .map(|elements: Vec<_>| elements.into_iter().map(String::from).collect())
        .parse_next(input)
}

pub fn parse_recipe(mut recipe: &str) -> Vec<&str> {
    let order_location = (parse_order_location, parse_stock_items).parse_next(&mut recipe);
    println!("{order_location:#?}");
    let order_location = order_location.unwrap();

    // println!("{bin:#?}");
    // let bin = bin.unwrap();
    println!("{order_location:#?}");
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simples() {
        // ftp/cars.parquet <format=parquet> [ 100 * { make: 'Honda', model: Name } ];
        let value = parse_recipe("store.section.shelf [ 100 *  {i} ]");
        assert!(false)
        // assert_eq!(value, String::from("store.section"));
    }
}
