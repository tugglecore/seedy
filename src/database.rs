pub mod duck_store;

use crate::{Seeder, Store};
use crate::recipe::*;
use async_trait::async_trait;
pub use duck_store::*;


pub struct Column {
    column_name: String,
    referent: Option<(String, String, String, String)>
}

pub struct Table {
    table_name: String,
    columns: Vec<Column>
}

pub struct DatabaseSeeder {
    store_name: String,
    store: Box<dyn Store>,
}

impl From<DatabaseSeeder> for Box<dyn Seeder> {
    fn from(value: DatabaseSeeder) -> Self {
        Box::new(value)
    }
}

impl DatabaseSeeder {
    pub async fn new(store: Box<dyn Store>, store_name: String) -> Self {
        Self { store, store_name }
    }

    pub fn for_duckdb(connection: duckdb::Connection) -> Self {
        let store = Box::new(DuckStore::from_connection(connection));
        Self {
            store_name: String::from("duckdb"),
            store,
        }
    }
}

#[async_trait]
impl Seeder for DatabaseSeeder {
    fn store_name(&self) -> String {
        self.store_name.clone()
    }

    fn has_location(&self, stock_destination: &[String]) -> bool {
        true
    }

    async fn seed(&self, stock_order: Order) {
        self.store.plant(&Order::default()).await;
    }

    async fn get_schema(&self, item: Vec<String>) -> Option<u16> {
        Some(17)
    }
}
