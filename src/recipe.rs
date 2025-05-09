use winnow::{Parser, Result};
use winnow::ascii::alpha1;
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::token::one_of;
use winnow::combinator::{opt, repeat, separated, terminated};
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
    pub store_name: String,
    pub seeds: Vec<Seed>,
    pub bin: String
}

fn parse_store<'s>(input:  &mut &'s str) -> Result<Vec<&'s str>> {
    let text = separated(0.., alpha1, one_of(['/', '.'])).parse_next(input)?;

    Ok(text)
}

pub fn parse_recipe(mut recipe: &str) -> Vec<&str> {
    let bin = parse_store.parse_next(&mut recipe);

    println!("{bin:#?}");
    let bin = bin.unwrap();
    println!("{bin:#?}");
    bin
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simples() {
        let value = parse_recipe("store.section.shelf");
        println!("{value:#?}");
        assert!(false)
        // assert_eq!(value, String::from("store.section"));
    }
}
