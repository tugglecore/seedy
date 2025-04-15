use duckdb::{params, Connection, Result};
use duckdb::arrow::record_batch::RecordBatch;
use duckdb::arrow::util::pretty::print_batches;

trait Store {
    fn plant(&self, seed: &str);
}

impl Store for DuckStore {
    fn plant(&self, seed: &str) {
        self.connection.execute("INSERT INTO a VALUES (1);", []).unwrap();
    }
}

struct DuckStore {
    connection: Connection
}

impl DuckStore {
    pub fn new() -> Self {
        let connection = Connection::open_in_memory().unwrap();
        Self {
            connection
        }
    }

    pub fn from_connection(connection: Connection) -> Self {
        Self {
            connection
        }
    }
}

#[derive(Debug)]
struct Instruction {
    store_name: String
}

pub fn prep_recipe<T: Recipe>(recipe: T) -> Vec<Instruction> {
    vec![
        Instruction { store_name: String::from("duckdb") }
    ]
}

trait Seeder {
    fn store_name(&self) -> &str;
    fn seed(&self, instruction: Instruction);
}

struct DatabaseSeeder {
    store: Box<dyn Store>
}

impl DatabaseSeeder {
    fn new(store: Box<dyn Store>) -> Self {
        Self { store }
    }
}

impl Seeder for DatabaseSeeder {
    fn store_name(&self) -> &str {
        "duckdb"
    }

    fn seed(&self, instruction: Instruction) {
        self.store.plant("sdf")
    }
}

trait Recipe {}

impl Recipe for &str {}

trait StoreRegistry {
    fn build_seeders(&self) -> Vec<Box<dyn Seeder>>;
}

impl StoreRegistry for &str {
    fn build_seeders(&self) -> Vec<Box<dyn Seeder>> {
        vec![
            Box::new(
                DatabaseSeeder::new(
                    Box::new(DuckStore::new()))
            )
        ]
    }
}

impl StoreRegistry for Connection {
    fn build_seeders(&self) -> Vec<Box<dyn Seeder>> {
        let duckdb_connection = self.try_clone().unwrap();
        vec![
            Box::new(
                DatabaseSeeder::new(
                    Box::new(DuckStore::from_connection(duckdb_connection))
                )
            )
        ]
    }
}

struct Plower {
    seeders: Vec<Box<dyn Seeder>>
}

impl Plower {
    pub fn new<S: StoreRegistry>(store_registry: &S) -> Self {
        let seeders = store_registry.build_seeders();
        Self {
            seeders
        }
    }

    pub fn seed<R: Recipe>(&self, recipe: R) {
        let instructions = prep_recipe(recipe);

        for instruction in instructions {
            let seeder = self.seeders
                .iter()
                .find(|seeder| seeder.store_name() == instruction.store_name)
                .unwrap();

            seeder.seed(instruction);
        }
    }
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
        let _ = connection.execute_batch("CREATE TABLE a (id INTEGER);");

        let plower = Plower::new(&connection);
        let recipe = "patient [{1}, {2}]";
        plower.seed(recipe);

        let result: u8 = connection
            .prepare("Select * from a;")
            .unwrap()
            .query_row([], |row| row.get(0))
            .unwrap();
        assert_eq!(result, 1);
    }

    #[tokio::test]
    async fn test_seeding_s3() {
        let config = aws_config::from_env()
            .endpoint_url("http://localhost:4566")
            .load()
            .await;

        let client = aws_sdk_sqs::Client::new(&config);

        let queues = client.list_queues().send().await.unwrap();

        println!("Queues: {queues:#?}");
        assert!(false);
    }

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
