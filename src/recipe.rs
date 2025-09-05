use crate::Seeder;
use std::collections::HashMap;
use std::str::FromStr;
// use winnow::ascii::{alpha1, digit1, space0};
// use winnow::combinator::{
//     delimited, opt, preceded, repeat, separated, separated_pair, terminated,
// };
// use winnow::token::one_of;
// use winnow::{Parser, Result};
use chumsky::prelude::*;

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
//  Sharing Sets
//
//  Memory <count=100> [
//      { name, policy, client },
//  ];
//
//  Outcomes <count=100> [
//      Patients { name, policy },
//      Policy { policy, client }
//  ];
//
//  file [
//      aetna.csv { name:= "Koa",  state }
//  ];
//
//
//  file <count=100> [
//      aetna.csv { name, policy, client, state },
//  ];
//
//  file [
//      aetna.csv [
//          { name: 'sam', policy: 23, client: "FairCost", state: "Kentucky" },
//          { name: 'bob', policy: 99, client: "FairCost", state: "Ohio" }
//      ]
//  ];

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

#[derive(Debug, PartialEq)]
pub enum DataStock {
    Values(String),
    Count(u64),
    Inventory(Vec<DataStock>),
    Order {
        store_name: String,
        store_kind: String,
        modifiers: Vec<DataStock>,
        stock: Vec<DataStock>
    },
    Stock  {
        bin_name: String,
        records: Vec<DataStock>
    },
    Record {
        key: String,
        value: Option<String>,
        label: Option<String>
    }
}

fn parser<'src>() -> 
    impl Parser<'src, &'src str, DataStock, extra::Err<Cheap>> {
    let store = text::ascii::ident()
        .padded();


    let stock = text::ascii::ident().padded();

    store
        .then(stock.delimited_by(
        just("["),
        just("]")
        ))
        .padded()
        .map(|(store_name, bin_name)| {
            DataStock::Order {
                store_name: String::from(store_name),
                store_kind: String::from("Database"),
                modifiers: Vec::new(),
                stock: vec![DataStock::Stock {
                    bin_name:String::from(bin_name),
                    records: Vec::new()
                }]
            }
        })
}

// fn parser<'src>() -> impl Parser<'src, &'src str, DataStock> {
//     let store = text::ascii::ident()
//         .padded()
//         .map(|s: &str| DataStock::Order {
//             store_name: s.to_string(),
//             store_kind: String::from("Database"),
//             modifiers: Vec::new(),
//             stock: Vec::new()
//         });
//
//     let stock = just("a");
//
//
//     store
//         // .then(
// }

fn read_order(order: String) -> DataStock {
    let work = parser().parse(&order);

    match parser().parse(&order).into_result() {
        Ok(ast) => return ast,
        Err(parse_errs) => {
            parse_errs
            .into_iter()
            .for_each(|e| println!("Parse error: {}", e));
            panic!();
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! build_data_stock {
        (Stock { bin: $bin:expr }) => {
            DataStock::Stock {
                bin_name: String::from($bin),
                records: Vec::new()
            }
        };
        (Order { store: $name:expr }) => {
            DataStock::Order {
                store_name: String::from($name),
                store_kind: String::from("Database"),
                modifiers: Vec::new(),
                stock: Vec::new()
            }
        };
        (
            Order { store: $name:expr , stock: [ $($stocks:tt)+ ] } 
        ) => {
            DataStock::Order {
                store_name: String::from($name),
                store_kind: String::from("Database"),
                stock: vec![
                    build_data_stock!($($stocks)+)
                ],
                modifiers: Vec::new()
            }
        }
    }

    #[test]
    fn can_parse_a_store_name() {
        let actual_result = read_order(String::from("Academia"));
        assert_eq!(
            actual_result,
            build_data_stock!{
                Order { store: "Academia" }
            }
        );
    }

    #[test]
    fn can_parse_a_bin() {
        let inventory = String::from("
            Academia [ Student ]
        ");

        let actual_result = read_order(inventory);
        assert_eq!(
            actual_result,
            build_data_stock! {
                Order {
                    store: "Academia",
                    stock: [ Stock { bin: "Student" } ]
                }
            }
        );
    }

    // #[test_case(
    //     "a",
    //     build_order!(["a"]);
    //     "stock one identifier shelf"
    // )]
    // #[test_case(
    //     "a <b=c>",
    //     build_order!(["a"], [("b", "c")]);
    //     "stock shelf with modifier"
    // )]
    // // Note: a <b=c> [ 3 * ] is an error b/c of the trailing asterisk
    // #[test_case(
    //     "a <b=c> [ 3 ]",
    //     build_order!(["a"], [("b", "c")], 3);
    //     "stock shelf with specific count"
    // )]
    // #[test_case(
    //     "a <b=c> [ 14 * { d: e } ]",
    //     build_order!(
    //         ["a"], [("b", "c")], 14,
    //         [("d", "e")]
    //     );
    //     "stock shelf with requested stock"
    // )]
    // #[test_case(
    //     "store.section.shelf <a=b, c = d> [ 100 * { i:j, d:e, h:i } ]",
    //     build_order!(
    //         ["store", "section", "shelf"],
    //         [("a", "b"), ("c", "d")],
    //         100,
    //         [("i", "j"), ("d", "e"), ("h", "i")]
    //     );
    //     "kitchen sink example"
    // )]
    // fn parse_simples(stock_order: &str, expected_order: StockOrder) {
    //     let subject_order = process_order(stock_order).pop().unwrap();
    //
    //     assert_eq!(subject_order, expected_order);
    // }

}
