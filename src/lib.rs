use async_trait::async_trait;
use duckdb::arrow::record_batch::RecordBatch;
use duckdb::arrow::util::pretty::print_batches;
use duckdb::{Connection, Result, Row, params};
use tokio::runtime::Builder;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use std::str::FromStr;

/*
 *
 * service_scope { target <modifiers> [literals or generators(args)] }
 *
 */
#[async_trait]
trait Store {
    async fn plant(&self, seed: &str);
}

struct DuckStore {
    query_sender: Sender<String>,
    row_receiver: Mutex<Receiver<String>>,
    handle: std::thread::JoinHandle<()>,
}

impl DuckStore {
    pub fn from_connection(connection: Connection) -> Self {
        let (query_sender, mut query_receiver) = channel(100);
        let (row_sender, mut row_receiver) = channel(100);
        let row_receiver = Mutex::new(row_receiver);
        let handle = std::thread::spawn(move || {
            while let Some(query) = query_receiver.blocking_recv() {
                let result = connection.execute("INSERT INTO a VALUES (1);", []);
                row_sender.blocking_send(String::new());
            }
        });

        Self {
            handle,
            row_receiver,
            query_sender,
        }
    }

    fn new() -> Self {
        let connection = Connection::open_in_memory().unwrap();
        DuckStore::from_connection(connection)
    }
}

#[async_trait]
impl Store for DuckStore {
    async fn plant(&self, seed: &str) {
        self.query_sender.send(String::new()).await.unwrap();
        let mut receiver = self.row_receiver.lock().await;
        receiver.recv().await;
    }
}

struct SQSStore {
    client: aws_sdk_sqs::Client,
}

impl SQSStore {
    async fn new() -> Self {
        let config = aws_config::from_env()
            .endpoint_url("http://localhost:4566")
            .load()
            .await;
        let client = aws_sdk_sqs::Client::new(&config);
        Self { client }
    }
}

#[async_trait]
impl Store for SQSStore {
    async fn plant(&self, seed: &str) {}
}

#[derive(Debug)]
struct Instruction {
    store_name: String,
}

pub fn prep_recipe<T: Recipe>(recipe: T) -> Vec<Instruction> {
    vec![Instruction {
        store_name: String::from("duckdb"),
    }]
}

#[async_trait]
trait Seeder: Send + Sync {
    fn store_name(&self) -> &str;
    async fn seed(&self, instruction: Instruction);
}

struct DatabaseSeeder {
    store: Box<dyn Store + Send + Sync>,
}

impl DatabaseSeeder {
    fn new(store: Box<dyn Store + Send + Sync>) -> Self {
        Self { store }
    }

    fn for_duckdb(connection: Connection) -> Self {
        let store = Box::new(DuckStore::from_connection(connection));
        Self { store }
    }
}

#[async_trait]
impl Seeder for DatabaseSeeder {
    fn store_name(&self) -> &str {
        "duckdb"
    }

    async fn seed(&self, instruction: Instruction) {
        self.store.plant("sdf").await;
    }
}

struct SQSSeeder {
    store: aws_sdk_sqs::Client,
}

impl SQSSeeder {
    async fn new(url: url::Url) -> Self {
            let config = aws_config::from_env()
                .endpoint_url("sqs://localhost:4566")
                .load()
                .await;
            let store = aws_sdk_sqs::Client::new(&config);
        Self { store }
    }

    fn plant(&self, msg: &str) {}
}

#[async_trait]
impl Seeder for SQSSeeder {
    fn store_name(&self) -> &str {
        "sqs"
    }

    async fn seed(&self, instruction: Instruction) {
        self.plant("abstraction")
    }
}

trait Recipe {}

impl Recipe for &str {}

#[derive(Debug)]
enum StoreKind {
    sqs,
    DuckDB
}

impl FromStr for StoreKind {
    // FIXME: This is using duckdb result as the default
    // error type for all StoreKind conversions.
    type Err = Result<StoreKind>;

    fn from_str(store_name: &str) -> Result<Self, Self::Err> {
        let store = match store_name {
            "sqs" => Self::sqs,
            "duckdb" => Self::DuckDB,
            _ => panic!("unknown store")
        };

        Ok(store)
    }
}

#[async_trait]
trait StoreRegistry {
    async fn build_seeders(&self) -> Vec<Box<dyn Seeder>>;
}

#[async_trait]
impl StoreRegistry for str {
    async fn build_seeders(&self) -> Vec<Box<dyn Seeder>> {
        use StoreKind::*;

        let mut seeders = vec![];
        
        let url = url::Url::parse(self).unwrap();
        
        let store_kind = StoreKind::from_str(url.scheme()).unwrap();

        let seeder: Box<dyn Seeder> = match store_kind {
            sqs => Box::new(SQSSeeder::new(url).await),
            DuckDB => {
                let connection = Connection::open_in_memory().unwrap();
                Box::new(DatabaseSeeder::for_duckdb(connection))
            }
        };

        seeders.push(seeder);

        seeders
    }
}

// #[async_trait]
// impl StoreRegistry for Connection {
//     async fn build_seeders(&self) -> Vec<Box<dyn Seeder>> {
//         let duckdb_connection = self.try_clone().unwrap();
//         vec![Box::new(DatabaseSeeder::new(Box::new(
//             DuckStore::from_connection(duckdb_connection),
//         )))]
//     }
// }

struct Plower {
    seeders: Vec<Box<dyn Seeder>>,
}



impl Plower {
    // TODO: Implement trait objects to allow list of distinct types
    pub async fn new<S: StoreRegistry + ?Sized>(store_registry: &S) -> Self {
        let seeders = store_registry.build_seeders().await;

        Self { seeders }
    }

    pub async fn seed<R: Recipe>(&self, recipe: R) {
        let instructions = prep_recipe(recipe);

        for instruction in instructions {
            let seeder = self
                .seeders
                .iter()
                .find(|seeder| seeder.store_name() == instruction.store_name)
                .unwrap();

            seeder.seed(instruction).await;
        }
    }
    
    fn from_duckdb(connection: &Connection) -> Self {
        let mut seeders: Vec<Box<dyn Seeder>> = vec![];

        let duckdb_connection = connection.try_clone().unwrap();

        seeders.push(
            Box::new(
                DatabaseSeeder::for_duckdb(duckdb_connection)
            )
        );
    
        Self { seeders }
    }
   
}

pub fn add(left: u16, right: u16) -> u16 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn auto_discover_tables() {
        let connection = Connection::open_in_memory().unwrap();
        let _ = connection.execute_batch("CREATE TABLE a (id INTEGER);");

        let plower = Plower::from_duckdb(&connection);
        let recipe = "patient [{1}, {2}]";
        plower.seed(recipe).await;

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
        let queue_url = client
            .create_queue()
            .queue_name("crops")
            .send()
            .await
            .unwrap()
            .queue_url
            .unwrap();

        let plower = Plower::new("sqs://localhost:4566");
        // let recipe = "crops ['fertilizer']";
        // plower.seed(recipe);

        let queues = client.list_queues().send().await.unwrap();

        println!("Queues: {queues:#?}");
        // assert!(false);
    }

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
