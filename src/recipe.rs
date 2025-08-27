use crate::Seeder;
use std::collections::HashMap;
use std::str::FromStr;
use winnow::ascii::{alpha1, digit0, digit1, space0};
use winnow::combinator::{alt, delimited, opt, preceded, repeat, separated, separated_pair, terminated};
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
//
// Attempt 1:
// $law_student = pg.student [ 19 * { college: "Law" } ]
// pg.course [ { course: "Math" } ]
// pg.student [ { name: "Bill", college: "Law" } ]
// pg.enrollment [ { course: "Math", name: "Bill" } ]
// pg.enrollment [ 19 * { course: "Math", name: from_set($law_student) } ]
//
// Attempt 2:
// pg.enrollment [
//  19 * {
//          course: "Math",
//          name: ( pg.student [ 19 * { college: "Law" }, { name: "Bill", college: "Law" } ] )
//      }
//  ]
//
// Attempt 3:
// INSERT INTO Enrollment COLUMNS (name, course)
// VALUES (BILL, MATH), 20 * (_, Math)
//
// Attempt 4:
// INSERT Enrollment COLUMNS ((college, name), course)
// VALUES ((LAW, BILL), MATH), 20 * ((LAW, _), MATH)
// 
// Enrollment(Student)
// Student(Enrollment)
// (Enrollment(Student))
// (Student(Enrollment))
// Enrollment((Student), Course)
//
// INSERT Enrollment((Student), Course)
// VALUES ((LAW, BILL), MATH), 20 * ((LAW, _), MATH)
//
// INSERT Enrollment(( Student(college, name) ), course)
// VALUES ((Law, BILL), MATH), 20 * ((Law, _), MATH)
// 
// INSERT Enrollment(( Student(college, name) ), course)
// VALUES [
//      ((Law, BILL), MATH),
//      20 * ((Law, _), MATH)
// ]
//
// INSERT Enrollment{{ Student{college, name} }, course}
// VALUES [
//      ((Law, BILL), MATH),
//      20 * ((Law, _), MATH)
// ]
//
// INSERT Enrollment{Student{college, name}, course}
// VALUES [
//      ((Law, BILL), MATH),
//      20 * ((Law, _), MATH)
// ]
//
// INSERT { Enrollment: { course, name: { Student: { college, name } } } }
// VALUES [
//      { MATH, { LAW, BILL } },
//      20 * { MATH, { LAW, _ } },
//      10 * { MATH },
//      4
//  ]
//
//  INSERT { Tiprule: { ruletype, tipdetailid: { TipdetailRule: { publishTip } } } }
//  VALUES [
//      { Clinical, { True } },
//      { Clinical },
//      3
//  ]
//
//  Goal:
//  - 19 students
//  - 19 enrollments
//  - 1 college
//  - 1 course
//
//  Academia [
//      Student { college_title: "Law", name: "Bill" }
//      Enrollment { course_title: "Math", name }
//  ]
//  Academia <count=18> [
//      Student { college_title, name: @ },
//      Enrollment { course_title, name: @ }
//  ]
//
//
//  INSERT INTO COLLEGE (college_title) VALUES ('Law');
//  INSERT INTO Course (course_title) VALUES ('Math');
//  INSERT INTO Student (college_title, name) VALUES ('Law', 'Bill')
//  INSERT INTO Enrollment (course_title, name) VALUES('Math', 'Bi')
//  INSERT INTO Student (college_title, name)
//      VALUES ('Law', 'Bill'), ('Law', 'Sam'), ('Law', 'Tom')
//  INSERT INTO Enrollment (course_title, name)
//      VALUES ('Math', 'Bi'), ('Math', 'Sam'), ('Math', 'Tom')


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

// TODO: Need to create a Good struct to encapsulate
// all the different values will can have as goods
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stock {
    pub count: u32,
    pub goods: HashMap<String, String>,
}

#[derive(Clone, Debug, Default)]
pub struct Order {
    pub store_location: Vec<String>,
    pub store_modifiers: HashMap<String, String>,
    pub stock_kinds: Vec<Stock>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StockOrder {
    pub destination: Vec<String>,
    pub modifiers: HashMap<String, String>,
    pub requested_stock: Vec<Stock>,
}

fn parse_stock_items<'i>(input: &mut &'i str) -> Result<Stock> {
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

    let item_count = digit1.map(|s| u32::from_str(s).unwrap());

    delimited(
        (space0, "[", space0),
        (item_count, opt(preceded((space0, "*", space0), stock))),
        (space0, "]", space0),
    )
    .map(|(count, goods)| {
        Stock { 
            count, 
            goods: goods.unwrap_or_default()
        }
    })
    .parse_next(input)
}

fn parse_store_modifier<'i>(input: &mut &'i str) -> Result<HashMap<String, String>> {
    let store_modifiers = terminated(
        separated_pair(alpha1, (space0, "=", space0), alpha1),
        opt((space0, ",", space0)),
    )
    .map(|(k, v): (&str, &str)| (k.to_string(), v.to_string()));

    let changers = repeat::<_, _, HashMap<String, String>, _, _>(0.., store_modifiers);

    delimited((space0, "<", space0), changers, (space0, ">", space0)).parse_next(input)
}

fn parse_order_location<'i>(input: &mut &'i str) -> Result<Vec<String>> {
    separated(0.., alpha1, one_of(['/', '.']))
        .map(|elements: Vec<_>| elements.into_iter().map(String::from).collect())
        .parse_next(input)
}

pub fn process_order(mut recipe: &str) -> Vec<StockOrder> {
    let (destination, modifiers, stock) = (
        parse_order_location,
        opt(parse_store_modifier),
        opt(parse_stock_items),
    )
        .parse_next(&mut recipe)
        .unwrap();

    let stock_order = StockOrder {
        destination,
        modifiers: modifiers.unwrap_or_default(),
        // TODO: Add capability to parse various stocks
        // for same order. This will result in a Vec being
        // returned for our parser parse_stock_items and
        // allow capabilities such as
        // a [ {a: b}, 4 * { s: t } ]
        requested_stock: vec![stock.unwrap_or_default()],
    };

    vec![stock_order]
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
                requested_stock: vec![Stock::default()]
            }
        };
        ([$($store_parts:expr),*]) => {
            StockOrder {
                destination: vec![$(String::from($store_parts)),*],
                modifiers: HashMap::new(),
                requested_stock: vec![Stock::default()]
            }
        };
        ([$($store_parts:expr),*], [$(($mod_key:expr,$mod_val:expr)),+]) => {
            StockOrder {
                destination: vec![$(String::from($store_parts)),*],
                modifiers: HashMap::from(
                    [
                    $((
                            String::from($mod_key),
                            String::from($mod_val)
                    )),+
                    ]
                ),
                requested_stock: vec![Stock::default()]
            }
        };
        (
            [$($store_parts:expr),*],
            [$(($mod_key:expr,$mod_val:expr)),+],
            $count:expr
        ) => {
            StockOrder {
                destination: vec![$(String::from($store_parts)),*],
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
        (
            [$($store_parts:expr),*],
            [$(($mod_key:expr,$mod_val:expr)),+],
            $count:expr,
            [$(($col:expr,$val:expr)),+]
        ) => {
            StockOrder {
                destination: vec![$(String::from($store_parts)),*],
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
                        goods: HashMap::from(
                            [
                                $((
                                        String::from($col),
                                        String::from($val)
                                )),+
                            ]
                        )
                    }
                ]
            }
        };
    }

    #[test_case(
        "a",
        build_order!(["a"]);
        "stock one identifier shelf"
    )]
    #[test_case(
        "a <b=c>",
        build_order!(["a"], [("b", "c")]);
        "stock shelf with modifier"
    )]
    // Note: a <b=c> [ 3 * ] is an error b/c of the trailing asterisk
    #[test_case(
        "a <b=c> [ 3 ]",
        build_order!(["a"], [("b", "c")], 3);
        "stock shelf with specific count"
    )]
    #[test_case(
        "a <b=c> [ 14 * { d: e } ]",
        build_order!(
            ["a"], [("b", "c")], 14,
            [("d", "e")]
        );
        "stock shelf with requested stock"
    )]
    #[test_case(
        "store.section.shelf <a=b, c = d> [ 100 * { i:j, d:e, h:i } ]", 
        build_order!(
            ["store", "section", "shelf"],
            [("a", "b"), ("c", "d")],
            100,
            [("i", "j"), ("d", "e"), ("h", "i")]
        );
        "kitchen sink example"
    )]
    fn parse_simples(stock_order: &str, expected_order: StockOrder) {
        let subject_order = process_order(stock_order).pop().unwrap();

        assert_eq!(subject_order, expected_order);
    }
}
