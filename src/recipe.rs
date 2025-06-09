use crate::Seeder;
use std::collections::HashMap;
use std::str::FromStr;
use winnow::ascii::{alpha1, digit0, space0};
use winnow::combinator::{delimited, opt, repeat, separated, separated_pair, terminated};
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
//
// pg.enrollments [ 20 ]
//
// enroll 20 law students in a Math course with one named Bill:
// $law_student = pg.student [ 19 * { college: "Law" } ]
// pg.course [ { course: "Math" } ]
// pg.student [ { name: "Bill", college: "Law" } ]
// pg.enrollment [ { course: "Math", name: "Bill" } ]
// pg.enrollment [ 19 * { course: "Math", name: from_set($law_student) } ]

#[derive(Debug)]
pub struct Seed;

#[derive(Debug)]
pub struct Instruction {
    pub store_name: String,
}

pub fn prep_recipe(recipe: &str) -> Vec<Instruction> {
    // let order = Order {
    //     store_location: vec![String::from("duckdb")],
    //     store_modifiers: HashMap::new(),
    //     stock_kinds: vec![]
    // };
    // vec![order]
    vec![Instruction {
        store_name: String::from(recipe),
    }]
}

trait Recipe {}
impl Recipe for &str {}

type Bytes = Vec<u8>;

#[derive(Clone, Debug)]
pub enum Item {
    Digit(Bytes),
}

#[derive(Clone, Debug)]
pub struct Shipment {
    pub stock: Vec<Stock>,
}

#[derive(Clone, Debug, Default)]
pub struct Stock {
    pub target: Vec<String>,
    pub count: usize,
    pub goods: HashMap<String, Vec<Item>>,
}

#[derive(Clone, Debug, Default)]
pub struct Order {
    pub store_location: Vec<String>,
    pub store_modifiers: HashMap<String, String>,
    pub stock_kinds: Vec<Stock>,
}

#[derive(Clone, Debug, Default)]
pub struct StockOrder {
    pub destination: Vec<String>,
    pub modifiers: HashMap<String, String>,
    pub requested_stock: Vec<Stock>,
}

fn parse_stock_items<'i>(input: &mut &'i str) -> Result<(u32, HashMap<String, String>)> {
    let key_value = terminated(
        separated_pair(alpha1, (space0, ":", space0), alpha1),
        opt((space0, ",", space0)),
    )
    .map(|(k, v): (&str, &str)| (k.to_string(), v.to_string()));

    let stock_key_values = repeat::<_, _, HashMap<String, String>, _, _>(0.., key_value);

    let stock = delimited(
        (space0, "{", space0),
        stock_key_values,
        (space0, "}", space0),
    );

    let item_count = terminated(digit0, (space0, "*", space0)).map(|s| u32::from_str(s).unwrap());

    let stock_item_parser = (item_count, stock);

    let t = delimited(
        (space0, "[", space0),
        stock_item_parser,
        (space0, "]", space0),
    )
    .parse_next(input);

    t
}

fn parse_store_modifier<'i>(input: &mut &'i str) -> Result<HashMap<String, String>> {
    let store_modifiers = terminated(
        separated_pair(alpha1, (space0, "=", space0), alpha1),
        opt((space0, ",", space0)),
    )
    .map(|(k, v): (&str, &str)| (k.to_string(), v.to_string()));

    let changers = repeat::<_, _, HashMap<String, String>, _, _>(0.., store_modifiers);

    let t = delimited((space0, "<", space0), changers, (space0, ">", space0)).parse_next(input);

    t
}

fn parse_order_location<'i>(input: &mut &'i str) -> Result<Vec<String>> {
    separated(0.., alpha1, one_of(['/', '.']))
        .map(|elements: Vec<_>| elements.into_iter().map(String::from).collect())
        .parse_next(input)
}

pub fn process_order(mut recipe: &str) -> Vec<&str> {
    let (order_location, modifiers, stock) = (
        parse_order_location,
        parse_store_modifier,
        parse_stock_items,
    )
        .parse_next(&mut recipe)
        .unwrap();

    println!("{order_location:#?}");
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    macro_rules! build_order {
        () => {
            StockOrder {
                destination: vec![],
                modifiers: HashMap::new(),
                requested_stock: vec![]
            }
        };
        ([$($store_parts:expr),*]) => {
            StockOrder {
                destination:
                    vec![$(String::from($store_parts)),*],
                modifiers: HashMap::new(),
                requested_stock: vec![]
            }
        };
        ([$($store_parts:expr),*], [$(($mod_key:expr,$mod_val:expr)),+]) => {
            StockOrder {
                destination:
                    vec![$(String::from($store_parts)),*],
                modifiers: HashMap::from(
                    [
                    $((
                            String::from($mod_key),
                            String::from($mod_val)
                    )),+
                    ]
                ),
                requested_stock: vec![]
            }
        };
        (
            [$($store_parts:expr),*],
            [$(($mod_key:expr,$mod_val:expr)),+],
            $count:expr
        ) => {
            StockOr:der {
                destination:
                    vec![$(String::from($store_parts)),*],
                modifiers: HashMap::from(
                    [
                    $((
                            String::from($mod_key),
                            String::from($mod_val)
                    )),+
                    ]
                ),
                requested_stock: vec![
                    Stock {
                        count: $count,
                        goods: HashMap::new()
                    }
                ]
            }
        };
    }

    #[test_case(
        "store.section.shelf <a=b, c = d> [ 100 *  {i:j   ,d:e,  h :  i} ]", 
        build_order!(["store", "section", "shelf"], [("a", "b"), ("c", "d")]); 
        "kitchen sink example"
    )]
    fn parse_simples(stock_order: &str, expected_order: StockOrder) {
        let value = process_order(stock_order);
        println!("{expected_order:#?}");
        assert!(false)
        // assert_eq!(value, String::from("store.section"));
    }
}
