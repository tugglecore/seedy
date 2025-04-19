use async_trait::async_trait;
use duckdb::arrow::record_batch::RecordBatch;
use duckdb::arrow::util::pretty::print_batches;
use duckdb::{Connection, Result, Row, params};
use std::str::FromStr;
use tiberius::{AuthMethod, Client, Config, Query};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

/*
 *
 * service_scope { target <modifiers> [literals or generators(args)] }
 *
 */

/******************************************************************************************************************************************
 *************************************************************** STORE ********************************************************************
******************************************************************************************************************************************/
#[async_trait]
trait Store {
    async fn plant(&self, seed: &str);
    fn store_name(&self) -> String;
}

struct SqlServerStore {
    connection: Mutex<Client<Compat<TcpStream>>>,
}

impl SqlServerStore {
    async fn new(url: url::Url) -> Self {
        let mut config = Config::new();
        config.host("localhost");
        config.port(1433);
        config.authentication(AuthMethod::sql_server("SA", "Seedy2025"));
        config.trust_cert();

        let tcp = TcpStream::connect(config.get_addr()).await.unwrap();
        tcp.set_nodelay(true).unwrap();
        let connection = Client::connect(config, tcp.compat_write()).await.unwrap();
        let connection = Mutex::new(connection);
        Self { connection }
    }
}

#[async_trait]
impl Store for SqlServerStore {
    async fn plant(&self, seed: &str) {
        let mut query = Query::new("INSERT INTO tempdb.dbo.canvas VALUES (234)");
        // self.query_sender.send(String::new()).await.unwrap();
        // let mut receiver = self.row_receiver.lock().await;
        // receiver.recv().await;
        let mut conn = self.connection.lock().await;
        let results = query.execute(&mut conn).await.unwrap();
    }

    fn store_name(&self) -> String {
        String::from("ms")
    }
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

    fn store_name(&self) -> String {
        String::from("duckdb")
    }
}

/******************************************************************************************************************************************
 ************************************************************* INSTRUCTION ********************************************************************
******************************************************************************************************************************************/

#[derive(Debug)]
struct Instruction {
    store_name: String,
}

pub fn prep_recipe(recipe: &str) -> Vec<Instruction> {
    vec![Instruction {
        store_name: String::from(recipe),
    }]
}

trait Recipe {}
impl Recipe for &str {}

/******************************************************************************************************************************************
 ************************************************************* RECIPE ********************************************************************
******************************************************************************************************************************************/

#[async_trait]
trait Seeder: Send + Sync {
    fn store_name(&self) -> String;
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
    fn store_name(&self) -> String {
        self.store.store_name()
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
            .endpoint_url("http://localhost:4566")
            .load()
            .await;
        let store = aws_sdk_sqs::Client::new(&config);
        Self { store }
    }
}

#[async_trait]
impl Seeder for SQSSeeder {
    fn store_name(&self) -> String {
        String::from("sqs")
    }

    async fn seed(&self, instruction: Instruction) {
        let queue_url = self
            .store
            .get_queue_url()
            .queue_name("crops")
            .send()
            .await
            .unwrap()
            .queue_url
            .unwrap();

        self.store
            .send_message()
            .queue_url(queue_url)
            .message_body("fertilizer")
            .send()
            .await;
    }
}

/******************************************************************************************************************************************
 *************************************************** STORE KIND & REGISTRY ***************************************************************
******************************************************************************************************************************************/

#[derive(Debug)]
enum StoreKind {
    sqs,
    SqlServer,
    DuckDB,
}

impl FromStr for StoreKind {
    // FIXME: This is using duckdb result as the default
    // error type for all StoreKind conversions.
    type Err = Result<StoreKind>;

    fn from_str(store_name: &str) -> Result<Self, Self::Err> {
        let store = match store_name {
            "sqs" => Self::sqs,
            "ms" => Self::SqlServer,
            "duckdb" => Self::DuckDB,
            _ => panic!("unknown store"),
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

        println!("What is self: {self:#?}");
        let url = url::Url::parse(self).unwrap();

        let store_kind = StoreKind::from_str(url.scheme()).unwrap();

        println!("What is the store kind: {store_kind:#?}");
        let seeder: Box<dyn Seeder> = match store_kind {
            sqs => Box::new(SQSSeeder::new(url).await),
            SqlServer => {
                let store = SqlServerStore::new(url).await;
                Box::new(DatabaseSeeder::new(Box::new(store)))
            }
            DuckDB => {
                let connection = Connection::open_in_memory().unwrap();
                Box::new(DatabaseSeeder::for_duckdb(connection))
            }
        };

        seeders.push(seeder);

        seeders
    }
}

/******************************************************************************************************************************************
 *********************************************************** PLOWER ************************************************************************
******************************************************************************************************************************************/

struct Plower {
    seeders: Vec<Box<dyn Seeder>>,
}

impl Plower {
    // TODO: Implement trait objects to allow list of distinct types
    pub async fn new<S: StoreRegistry + ?Sized>(store_registry: &S) -> Self {
        let seeders = store_registry.build_seeders().await;

        Self { seeders }
    }

    pub async fn seed(&self, recipe: &str) {
        let instructions = prep_recipe(recipe);

        let seeder_count = self.seeders.len();
        println!("INstruction are: {instructions:#?}");
        println!("How many seeders we have: {seeder_count:#?}");
        let seeder_name = self.seeders.first().unwrap().store_name();
        println!("seeder name is: {seeder_name}");
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

        seeders.push(Box::new(DatabaseSeeder::for_duckdb(duckdb_connection)));

        Self { seeders }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn auto_discover_tables() {
        let connection = Connection::open_in_memory().unwrap();
        let _ = connection.execute_batch("CREATE TABLE a (id INTEGER);");

        let plower = Plower::from_duckdb(&connection);
        plower.seed("duckdb").await;

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

        client
            .send_message()
            .queue_url(queue_url)
            .message_body("fertilizer")
            .send()
            .await;

        // sqs://localhost:4566"
        let plower = Plower::new("sqs://localhost:4566").await;
        let recipe = "crops ['fertilizer']";
        plower.seed("sqs").await;

        let queues = client.list_queues().send().await.unwrap();

        let msgs = client
            .receive_message()
            .queue_url("http://sqs.us-east-1.localhost.localstack.cloud:4566/000000000000/crops")
            .max_number_of_messages(1)
            .send()
            .await
            .unwrap()
            .messages
            .unwrap();
        let actual_msg = msgs.first().unwrap().body().unwrap();

        assert_eq!(actual_msg, "fertilizer");
    }

    #[tokio::test]
    async fn test_sql_server() {
        let mut config = Config::new();
        config.host("localhost");
        config.port(1433);
        config.authentication(AuthMethod::sql_server("SA", "Seedy2025"));
        config.trust_cert();

        let tcp = TcpStream::connect(config.get_addr()).await.unwrap();
        tcp.set_nodelay(true).unwrap();
        let mut client = Client::connect(config, tcp.compat_write()).await.unwrap();

        let mut query = Query::new("INSERT INTO tempdb.dbo.canvas VALUES (234)");
        let results = query.execute(&mut client).await.unwrap();

        let plower = Plower::new("ms://sa:Seedy2025@localhost:1433").await;
        plower.seed("ms").await;
    }
}
