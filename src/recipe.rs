use crate::Seeder;
use std::collections::HashMap;
use winnow::ascii::{alpha1, digit1, multispace0, multispace1, space0};
use winnow::combinator::{
    alt, delimited, empty, opt, peek, preceded, repeat, separated, separated_pair, terminated,
    trace,
};
use winnow::prelude::*;

// url store.bin <modifiers> { count * [literals or generators(args)] }
// s3:products:/policy_13/
// s3:products:policy_13
// mssql:outcomesidentification:tipresult_new

// s3/products/policy_13/
// s3.products.policy_13
// mssql.outcomesidentification.tipresult_new
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
pub struct Attribute {
    key: String,
    value: Option<String>,
    label: Option<String>
}

#[derive(Debug, PartialEq)]
pub struct Record {
    attributes: Vec<Attribute>
}

#[derive(Debug, PartialEq)]
pub struct Stok {
    bin_name: String,
    records: Vec<Record>
}

#[derive(Debug, PartialEq)]
pub struct Request {
    store_name: String,
    store_kind: String,
    modifiers: Vec<String>,
    stock: Vec<Stok>
}

#[derive(Debug, PartialEq)]
pub struct Inventory {
    request: Vec<Request>
}


fn parse_record<'i>(input: &mut &'i str) -> winnow::Result<Record> {
    let whitespace = |t| (multispace0, t, multispace0);

    let attribute = separated_pair(
        alpha1.map(String::from),
        whitespace(":"),
        alpha1.map(String::from),
    )
    .map(|(key, value)| Attribute {
        key,
        value: Some(value),
        label: None,
    });

    delimited(
        whitespace("{"),
        separated(0.., attribute, ","),
        (opt(whitespace(",")), whitespace("}")),
    )
    .map(|attributes| Record { attributes })
    .parse_next(input)
}

fn parse_stock<'i>(input: &mut &'i str) -> winnow::Result<Stok> {
    let whitespace = |t| (multispace0, t, multispace0);

    let multiple_records = delimited(whitespace("["), parse_record, whitespace("]"));

    let bin_name = alpha1.map(String::from).parse_next(input)?;

    let records = alt((
        multiple_records.map(|ds| vec![ds]),
        parse_record.map(|ds| vec![ds]),
        empty.map(|_| Vec::new()),
    ))
    .parse_next(input)?;

    Ok(Stok { bin_name, records })
}

fn parser<'src>(input: &mut &'src str) -> winnow::Result<Request> {
    let whitespace = |t| (multispace0, t, multispace0);

    let store_name = alpha1.map(String::from).parse_next(input)?;

    let stock = delimited(whitespace("["), parse_stock, whitespace("]")).parse_next(input)?;

    Ok(Request {
        store_name,
        stock: vec![stock],
        store_kind: String::from("Database"),
        modifiers: vec![],
    })
}

fn read_inventory(mut inventory: String) -> Request {
    parser(&mut inventory.as_str()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! build_data_stock {
        ({ bin: $bin:expr }) => {
            Stok {
                bin_name: String::from($bin),
                records: vec![]
            }
        };
        ([ { $key:expr, $val:expr } ] ) => {
            Record {
                attributes: vec![
                   Attribute {
                        key: String::from($key),
                        value: Some(String::from($val)),
                        label: None
                    }
                ]
            }
        };
        ({ bin: $bin:expr, records: [ $($records:tt)+ ]}) => {
            Stok {
                bin_name: String::from($bin),
                records: vec![
                    build_data_stock!($($records)+)
                ]
            }
        };
        ({ store: $name:expr , stock: [ $($stocks:tt)+ ] }) => {
            Request {
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
    fn can_parse_a_single_bin() {
        let inventory = "Academia [ Student ]";
        let actual_data_stock = read_inventory(String::from(inventory));

        assert_eq!(
            actual_data_stock,
            build_data_stock! {
                {
                    store: "Academia",
                    stock: [{ bin: "Student" }]
                }
            }
        );
    }

    #[test]
    fn can_parse_a_single_record() {
        let inventory = "Academia [ Student { key:value, }]";
        let actual_data_stock = read_inventory(String::from(inventory));

        assert_eq!(
            actual_data_stock,
            build_data_stock! {
                {
                    store: "Academia",
                    stock: [
                        {
                            bin: "Student",
                            records: [
                                [{"key","value"}]
                            ]
                        }
                    ]
                }
            }
        );
    }

    // #[test]
    // fn can_parse_a_store_name() {
    //     let actual_result = read_order(String::from("Academia"));
    //     assert_eq!(
    //         actual_result,
    //         build_data_stock!{
    //             Order { store: "Academia" }
    //         }
    //     );
    // }
    //
    // #[test]
    // fn can_parse_a_bin() {
    //     let inventory = String::from("
    //         Academia [ Student ]
    //     ");
    //
    //     let actual_result = read_order(inventory);
    //     assert_eq!(
    //         actual_result,
    //         build_data_stock! {
    //             Order {
    //                 store: "Academia",
    //                 stock: [ Stock { bin: "Student" } ]
    //             }
    //         }
    //     );
    // }

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
