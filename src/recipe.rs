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
    label: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct Record {
    attributes: Vec<Attribute>,
}

#[derive(Debug, PartialEq)]
pub struct Stok {
    bin_name: String,
    records: Vec<Record>,
}

#[derive(Debug, PartialEq)]
pub struct Request {
    store_name: String,
    store_kind: String,
    modifiers: Vec<String>,
    stock: Vec<Stok>,
}

#[derive(Debug, PartialEq)]
pub struct StockOrder {
    requests: Vec<Request>,
}

#[derive(Debug, PartialEq)]
pub struct DataStock {
    stock_order_ast: StockOrder,
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

fn parse_request<'src>(input: &mut &'src str) -> winnow::Result<Request> {
    let whitespace = |t| (multispace0, t, multispace0);

    let store_name = delimited(multispace0, alpha1, multispace0)
        .map(String::from)
        .parse_next(input)?;

    let stock = delimited(
        whitespace("["),
        repeat(0.., delimited(multispace0, parse_stock, multispace0)),
        whitespace("]"),
    )
    .parse_next(input)?;

    Ok(Request {
        store_name,
        stock,
        store_kind: String::from("Database"),
        modifiers: vec![],
    })
}
fn parser<'src>(input: &mut &'src str) -> winnow::Result<StockOrder> {
    let requests =
        repeat(0.., delimited(multispace0, parse_request, multispace0)).parse_next(input)?;

    Ok(StockOrder { requests })
}

fn read_inventory(mut inventory: String) -> StockOrder {
    parser(&mut inventory.as_str()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use test_case::test_case;

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
        ({ store: $name:expr , stock: [ $($stocks:tt),+ ] }) => {
            Request {
                store_name: String::from($name),
                store_kind: String::from("Database"),
                stock: vec![
                    $(build_data_stock!($stocks)),+
                ],
                modifiers: Vec::new()
            }
        };
        ($($order:tt);*$(;)?) => {
            StockOrder {
                requests: vec![$(build_data_stock!($order)),+]
            }
        }
    }

    #[test_case(
        "Academia [ Student ]",
        build_data_stock! [
            {
                store: "Academia",
                stock: [{ bin: "Student" }]
            };
        ];
        "a single bin"
    )]
    #[test_case(
        "Academia [ Student College ]",
        build_data_stock! [
            {
                store: "Academia",
                stock: [{ bin: "Student" }, { bin: "College" }]
            };
        ];
        "multiple bins"
    )]
    #[test_case(
        "Academia [ Student { key:value }]",
        build_data_stock! [
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
            };
        ];
        "a single record with a single attribute"
    )]
    // #[test_case(
    //     "Academia [ Student { key:value }]",
    //     build_data_stock! [
    //         {
    //             store: "Academia",
    //             stock: [
    //                 {
    //                     bin: "Student",
    //                     records: [
    //                         [{"key","value"}]
    //                     ]
    //                 }
    //             ]
    //         };
    //     ];
    //     "a single record with multiple attributes"
    // )]
    #[test_case(
        "
            Academia [ 
                Student { key:value, } College { key: value }
            ]
        ",
        build_data_stock! [
            {
                store: "Academia",
                stock: [
                    { bin: "Student", records: [[{"key","value"}]] },
                    { bin: "College", records: [[{"key", "value"}]] }
                ]
            };
        ];
        "multiple records"
    )]
    #[test_case(
        "
            Academia [ 
                Student { key:value, } College { key: value }
            ]
            LMS [ Grades { key: value } ]
        ",
        build_data_stock![
            {
                store: "Academia",
                stock: [
                    { bin: "Student", records: [[{"key","value"}]] },
                    { bin: "College", records: [[{"key", "value"}]] }
                ]
            };
            {
                store: "LMS",
                stock: [
                    { bin: "Grades", records: [[{"key","value"}]] }
                ]
            };
        ];
        "multiple_stores"
    )]
    fn can_parse(stock_order: &str, expected_stock_order: StockOrder) {
        let actual_data_stock = read_inventory(String::from(stock_order));

        assert_eq!(actual_data_stock, expected_stock_order);
    }
}
