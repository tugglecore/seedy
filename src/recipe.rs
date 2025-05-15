use std::str::FromStr;
use winnow::ascii::{alpha1, digit0, space0};
use winnow::combinator::{delimited, opt, repeat, separated, terminated};
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

fn parse_record<'i>(input: &mut &'i str) -> Result<u32> {
    // let w = (space0,
    //     digit0.map(|s| u32::from_str(s).unwrap()),
    //     space0
    // ).parse_next(input);

    let w = space0
        .map_err(|e| "s")
        .parse_next(input);
    println!("{w:#?}");
    Ok(w.unwrap())
}

fn parse_stock_items<'i>(input: &mut &'i str) -> Result<u32> {
    let t = delimited("[", parse_record, "]").parse_next(input);
    println!("{t:#?}");
    t
}

fn parse_order_location<'i>(input: &mut &'i str) -> Result<Vec<String>> {
    separated(0.., alpha1, one_of(['/', '.']))
        .map(|elements: Vec<_>| elements.into_iter().map(String::from).collect())
        .parse_next(input)
}

pub fn parse_recipe(mut recipe: &str) -> Vec<&str> {
    let order_location = (parse_order_location, parse_stock_items)
        .parse_next(&mut recipe)
        .unwrap();

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
        let value = parse_recipe("store.section.shelf [ 100 ]");
        assert!(false)
        // assert_eq!(value, String::from("store.section"));
    }
}
