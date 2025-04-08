use duckdb::{params, Connection, Result};
use duckdb::arrow::record_batch::RecordBatch;
use duckdb::arrow::util::pretty::print_batches;

trait Locator {
    fn build_registry<S: Store>(&self) -> StoreRegistry<S>;
}

impl Locator for &str {
    fn build_registry<S: Store> (&self) -> StoreRegistry<S> {
        let store = DuckStore::new();
        let r = StoreRegistry::new(vec![store]);
        r
    }
}

trait Store {
    fn what () -> u8 { 7 }
}

impl Store for DuckStore {}

struct DuckStore {}

impl DuckStore {
    pub fn new() -> Self {
        Self {}
    }
}

struct Instruction {}

pub fn prep_recipe<T: Recipe>(recipe: T) -> Vec<Instruction> {
    vec![]
}

trait Seeder {}

trait Recipe {}

impl Recipe for &str {}

struct StoreRegistry<S: Store> {
    stores: Vec<S>
}

impl<S: Store> StoreRegistry<S> {
    pub fn new(stores: Vec<S>) -> Self {
        Self {
            stores
        }
    }
}

struct Plower<S: Store> {
    store_registry: StoreRegistry<S>
}

impl<S: Store> Plower<S> {
    pub fn new<L: Locator>(store_locator: L) -> Self {
        let store_registry = store_locator.build_registry();
        Self {
            store_registry
        }
    }

    pub fn seed<R: Recipe>(&self, recipe: R) {
        let instructions = prep_recipe(recipe);

        for instruction in instructions {}
    }
}

pub fn seed_recipe(target_name: &str, seed: u16)  {

}

pub fn add(left: u16, right: u16) -> u16 {
    left + right
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_discover_tables() {
        let connection = Connection::open_in_memory().unwrap();
        let _ = connection.execute_batch("CREATE TABLE patient (id INTEGER);");

        let store_registry = "duckdb";
        // let plower = Plower::new(store_registry);
        // let recipe = "patient [{1}, {2}]";
        // plower.seed(recipe);
    }

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
