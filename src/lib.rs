pub mod recipe;

use arrow_array::record_batch;
use async_trait::async_trait;
use duckdb::Result;
use mongodb::{
    Collection,
    bson::{Document, doc},
};
use object_store::ObjectStore;
use object_store::local::LocalFileSystem;
use parquet::arrow::async_writer::AsyncArrowWriter;
use rdkafka::client::DefaultClientContext;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use redis::AsyncCommands;
use redis::aio::MultiplexedConnection;
use russh::*;
use russh_sftp::client::SftpSession;
use sqlx::AnyConnection;
use sqlx::any::install_default_drivers;
use sqlx::prelude::*;
use std::io::Write;
use std::str::FromStr;
use suppaftp::FtpStream;
use tiberius::{AuthMethod, Client, Config, Query};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};
use recipe::*;

/*
 *
 * service_scope { target <modifiers> [literals or generators(args)] }
 *
 * file { course.parquet <format: parquet> [ { topic: Chemistry } ] }
 *
 * field_identifier <modifiers> { [literals or generators(args)] }
 *
 * field <modifiers>? plot <modifiers>? { count? * [literals or generators(args)] }
 *
 */

/******************************************************************************************************************************************
 *************************************************************** STORE ********************************************************************
******************************************************************************************************************************************/

#[async_trait]
trait Store: Send + Sync {
    async fn plant(&self, seed: &str);
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
        let query = Query::new(
            "
                TRUNCATE TABLE tempdb.dbo.canvas;
                INSERT INTO tempdb.dbo.canvas VALUES (234);
            ",
        );
        let mut conn = self.connection.lock().await;
        let results = query.execute(&mut conn).await.unwrap();
    }
}

struct DuckStore {
    query_sender: Sender<String>,
    row_receiver: Mutex<Receiver<String>>,
    handle: std::thread::JoinHandle<()>,
}

impl DuckStore {
    pub fn from_connection(connection: duckdb::Connection) -> Self {
        let (query_sender, mut query_receiver) = channel(100);
        let (row_sender, row_receiver) = channel(100);
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
        let connection = duckdb::Connection::open_in_memory().unwrap();
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

#[async_trait]
impl<T: ObjectStore> Store for T {
    async fn plant(&self, seed: &str) {
        let batch = record_batch!(("a", Int32, [1, 2, 3])).unwrap();

        let mut buffer: Vec<u8> = vec![];
        let mut writer = AsyncArrowWriter::try_new(&mut buffer, batch.schema(), None).unwrap();

        writer.write(&batch).await.unwrap();
        writer.close().await.unwrap();

        let path = object_store::path::Path::from("/course.parquet");
        let payload = object_store::PutPayload::from(buffer);
        let put_result = self.put(&path, payload).await.unwrap();
    }
}

struct FtpStore {
    client: Mutex<FtpStream>,
}

impl FtpStore {
    fn new() -> Self {
        let mut client = FtpStream::connect("127.0.0.1:2121").unwrap();
        client.login("t", "k").unwrap();

        let client = Mutex::new(client);
        Self { client }
    }
}

#[async_trait]
impl Store for FtpStore {
    async fn plant(&self, seed: &str) {
        let r = b"it happens";

        self.client
            .lock()
            .await
            .put_file("happens.txt", &mut r.as_slice())
            .unwrap();
    }
}

struct SpamStore {
    connection: Mutex<AnyConnection>,
}

impl SpamStore {
    async fn new(url: url::Url) -> Self {
        install_default_drivers();

        let mut connection =
            AnyConnection::connect("postgres://postgres:mysecretpassword@localhost/postgres")
                .await
                .unwrap()
                .into();

        Self { connection }
    }
}

#[async_trait]
impl Store for SpamStore {
    async fn plant(&self, seed: &str) {
        use sqlx::Connection;
        let mut connection = self.connection.lock().await;

        let _ = connection
            .execute("INSERT INTO show VALUES ('tmnt')")
            .await
            .unwrap();
    }
}

struct SqsStore {
    client: aws_sdk_sqs::Client,
}
impl SqsStore {
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
impl Store for SqsStore {
    async fn plant(&self, seed: &str) {
        let queue_url = self
            .client
            .get_queue_url()
            .queue_name("crops")
            .send()
            .await
            .unwrap()
            .queue_url
            .unwrap();

        self.client
            .send_message()
            .queue_url(queue_url)
            .message_body("fertilizer")
            .send()
            .await;
    }
}

struct KafkaStore {
    client: FutureProducer,
}

impl KafkaStore {
    async fn new() -> Self {
        let client: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", "localhost:9092")
            .create()
            .unwrap();

        Self { client }
    }
}

#[async_trait]
impl Store for KafkaStore {
    async fn plant(&self, seed: &str) {
        let payload = String::from("crabgrass");
        let record: FutureRecord<String, String> = FutureRecord::to("garden").payload(&payload);

        let _ = self
            .client
            .send(record, std::time::Duration::from_millis(4000))
            .await
            .unwrap();
    }
}

struct SftpStore {
    session: SftpSession,
}

impl SftpStore {
    async fn new() -> Self {
        struct Client;

        impl client::Handler for Client {
            type Error = eyre::Report;

            async fn check_server_key(
                &mut self,
                server_public_key: &russh::keys::PublicKey,
            ) -> Result<bool, Self::Error> {
                Ok(true)
            }
        }

        let config = russh::client::Config::default();
        let sh = Client {};
        let mut session =
            russh::client::connect(std::sync::Arc::new(config), ("localhost", 22), sh)
                .await
                .unwrap();

        session.authenticate_password("foo", "pass").await.unwrap();

        let channel = session.channel_open_session().await.unwrap();
        channel.request_subsystem(true, "sftp").await.unwrap();
        let session = SftpSession::new(channel.into_stream()).await.unwrap();

        Self { session }
    }
}

#[async_trait]
impl Store for SftpStore {
    async fn plant(&self, seed: &str) {
        let f = self.session.create("test/mystery").await.unwrap();
    }
}

struct RedisStore {
    connection: Mutex<MultiplexedConnection>,
}

impl RedisStore {
    async fn new() -> Self {
        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let connection = client
            .get_multiplexed_async_connection()
            .await
            .unwrap()
            .into();

        Self { connection }
    }
}

#[async_trait]
impl Store for RedisStore {
    async fn plant(&self, seed: &str) {
        self.connection
            .lock()
            .await
            .set::<_, _, String>("lawn", "turf")
            .await
            .unwrap();
    }
}

struct MongoStore {
    client: mongodb::Client,
}

impl MongoStore {
    async fn new() -> Self {
        let client = mongodb::Client::with_uri_str("mongodb://127.0.0.1/")
            .await
            .unwrap();
        Self { client }
    }
}

#[async_trait]
impl Store for MongoStore {
    async fn plant(&self, seed: &str) {
        let doc = doc! { "brand": "LG" };
        let r = self
            .client
            .database("electronics")
            .collection::<Document>("tv")
            .insert_one(&doc)
            .await
            .unwrap();
    }
}

/******************************************************************************************************************************************
 ************************************************************* SEEDER ********************************************************************
******************************************************************************************************************************************/

#[async_trait]
trait Seeder: Send + Sync {
    fn store_name(&self) -> String;
    async fn seed(&self, instruction: Instruction);
}

struct DatabaseSeeder {
    store_name: String,
    store: Box<dyn Store>,
}

impl From<DatabaseSeeder> for Box<dyn Seeder> {
    fn from(value: DatabaseSeeder) -> Self {
        Box::new(value)
    }
}

impl DatabaseSeeder {
    async fn new(store: Box<dyn Store>, store_name: String) -> Self {
        Self { store, store_name }
    }

    fn for_duckdb(connection: duckdb::Connection) -> Self {
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

    async fn seed(&self, instruction: Instruction) {
        self.store.plant("sdf").await;
    }
}

struct MsgSeeder {
    store_name: String,
    store: Box<dyn Store>,
}

impl MsgSeeder {
    async fn new(store: Box<dyn Store>, store_name: String) -> Self {
        Self { store, store_name }
    }
}

#[async_trait]
impl Seeder for MsgSeeder {
    fn store_name(&self) -> String {
        self.store_name.clone()
    }

    async fn seed(&self, instruction: Instruction) {
        self.store.plant("fs").await
    }
}

impl From<MsgSeeder> for Box<dyn Seeder> {
    fn from(value: MsgSeeder) -> Self {
        Box::new(value)
    }
}

struct FileSeeder {
    name: String,
    store: Box<dyn Store>, // path: PathBuf
}

impl FileSeeder {
    async fn new(store: Box<dyn Store>, name: String) -> Self {
        Self { name, store }
    }

    async fn seed_parquet(&self, instruction: Instruction) {
        self.store.plant("something").await;
    }
}

#[async_trait]
impl Seeder for FileSeeder {
    fn store_name(&self) -> String {
        self.name.clone()
    }

    async fn seed(&self, instruction: Instruction) {
        let format = String::from("parquet");

        match format.as_str() {
            "parquet" => self.seed_parquet(instruction).await,
            _ => panic!("unknown file format"),
        };
    }
}

impl From<FileSeeder> for Box<dyn Seeder> {
    fn from(value: FileSeeder) -> Self {
        Box::new(value)
    }
}

/******************************************************************************************************************************************
 *************************************************** STORE KIND & REGISTRY ***************************************************************
******************************************************************************************************************************************/

// #[derive(Debug)]
enum StoreKind {
    Msg(Box<dyn Store>, String),
    File(Box<dyn Store>, String),
    Database(Box<dyn Store>, String),
}

impl StoreKind {
    async fn new(url: url::Url) -> Self {
        let store_name = url.scheme();

        match store_name {
            "sqs" => {
                let store = SqsStore::new().await;
                Self::Msg(Box::new(store), String::from("sqs"))
            }
            "kafka" => {
                let store = KafkaStore::new().await;
                Self::Msg(Box::new(store), String::from("kafka"))
            }
            "file" => {
                let store = Box::new(LocalFileSystem::new_with_prefix(url.path()).unwrap());
                Self::File(store, String::from("file"))
            }
            "s3" => {
                let store = object_store::aws::AmazonS3Builder::from_env()
                    .with_bucket_name("farm")
                    .with_access_key_id("test")
                    .with_secret_access_key("test")
                    .with_endpoint("http://localhost:4566")
                    .with_allow_http(true)
                    .build()
                    .unwrap();

                Self::File(Box::new(store), String::from("S3"))
            }
            "ftp" => {
                let store = Box::new(FtpStore::new());
                Self::File(store, String::from("ftp"))
            }
            "sftp" => {
                let store = Box::new(SftpStore::new().await);

                Self::File(store, String::from("sftp"))
            }
            "ms" => {
                let store = Box::new(SqlServerStore::new(url).await);
                Self::Database(store, String::from("ms"))
            }
            "postgres" => {
                let store = SpamStore::new(url).await;
                Self::Database(Box::new(store), String::from("postgres"))
            }
            "duckdb" => {
                let connection = duckdb::Connection::open_in_memory().unwrap();
                let store = Box::new(DuckStore::from_connection(connection));
                Self::Database(store, String::from("duckdb"))
            }
            "redis" => {
                let store = RedisStore::new().await;
                Self::Database(Box::new(store), String::from("redis"))
            }
            "mongodb" => {
                let store = MongoStore::new().await;
                Self::Database(Box::new(store), String::from("mongodb"))
            }
            _ => panic!("unknown store: received store {store_name}"),
        }
    }
}

#[async_trait]
trait StoreRegistry {
    async fn build_seeders(&self) -> Vec<Box<dyn Seeder>>;
}

#[async_trait]
impl StoreRegistry for &str {
    async fn build_seeders(&self) -> Vec<Box<dyn Seeder>> {
        use StoreKind::*;

        let mut seeders = vec![];

        let url = url::Url::parse(self).unwrap();

        let store_kind = StoreKind::new(url).await;

        // TODO: remove boxing from each branch. Potential solution is to
        // implement Into<Box> for every seeder. This will only result in
        // a code asthetic improvement
        let seeder: Box<dyn Seeder> = match store_kind {
            File(store, name) => FileSeeder::new(store, name).await.into(),
            Msg(store, name) => MsgSeeder::new(store, name).await.into(),
            Database(store, name) => DatabaseSeeder::new(store, name).await.into(),
        };

        seeders.push(seeder);

        seeders
    }
}

#[async_trait]
impl StoreRegistry for &String {
    async fn build_seeders(&self) -> Vec<Box<dyn Seeder>> {
        self.as_str().build_seeders().await
    }
}

#[async_trait]
impl StoreRegistry for String {
    async fn build_seeders(&self) -> Vec<Box<dyn Seeder>> {
        self.as_str().build_seeders().await
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
    pub async fn new<S: StoreRegistry>(store_registry: S) -> Self {
        let seeders = store_registry.build_seeders().await;

        Self { seeders }
    }

    pub async fn seed(&self, recipe: &str) {
        let instructions = prep_recipe(recipe);

        let seeder_count = self.seeders.len();
        let seeder_name = self.seeders.first().unwrap().store_name();
        for instruction in instructions {
            let seeder = self
                .seeders
                .iter()
                .find(|seeder| seeder.store_name() == instruction.store_name)
                .unwrap();

            seeder.seed(instruction).await;
        }
    }

    fn from_duckdb(connection: &duckdb::Connection) -> Self {
        let mut seeders: Vec<Box<dyn Seeder>> = vec![];

        let duckdb_connection = connection.try_clone().unwrap();

        seeders.push(Box::new(DatabaseSeeder::for_duckdb(duckdb_connection)));

        Self { seeders }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream::StreamExt;
    use libunftp::Server;
    use mongodb::{
        Collection,
        bson::{Document, doc},
    };
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
    use rdkafka::admin::{AdminClient, AdminOptions, NewTopic, TopicReplication};
    use rdkafka::config::FromClientConfig;
    use rdkafka::consumer::Consumer;
    use rdkafka::consumer::stream_consumer::StreamConsumer;
    use rdkafka::message::Message;
    use rdkafka::topic_partition_list::Offset;
    use rdkafka::topic_partition_list::TopicPartitionList;
    use sqlx::Connection;
    use sqlx::Executor;
    use sqlx::postgres::PgConnection;
    use std::time::Duration;
    use suppaftp::AsyncFtpStream;
    use suppaftp::list;
    use tokio::time::timeout;
    use unftp_sbe_fs::ServerExt;

    async fn spin_up_ftp_server() {
        let handle = tokio::spawn(async {
            let tmpdir = tempfile::TempDir::new().unwrap();
            let tmpdir = tmpdir.path().to_str().unwrap().to_string();
            let server = Server::with_fs(tmpdir).build().unwrap();

            server.listen("127.0.0.1:2121").await.unwrap();
        });

        let healthcheck = tokio::spawn(async {
            let ftp_stream = AsyncFtpStream::connect("127.0.0.1:2121").await.unwrap();
        });

        timeout(Duration::from_millis(3000), healthcheck)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn auto_discover_tables() {
        let connection = duckdb::Connection::open_in_memory().unwrap();
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
    async fn test_seeding_sqs() {
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
    async fn test_seeding_s3() {
        let expected_batch = record_batch!(("a", Int32, [1, 2, 3])).unwrap();
        let config = aws_config::from_env()
            .endpoint_url("http://s3.localhost.localstack.cloud:4566")
            .load()
            .await;
        let client = aws_sdk_s3::Client::new(&config);
        let bucket_location = client
            .create_bucket()
            .bucket("farm")
            .send()
            .await
            .unwrap()
            .location
            .unwrap();

        let plower = Plower::new("s3://localhost:4566").await;
        // let recipe = "crops ['fertilizer']";
        plower.seed("S3").await;

        let store = object_store::aws::AmazonS3Builder::from_env()
            .with_bucket_name("farm")
            .with_access_key_id("test")
            .with_secret_access_key("test")
            .with_endpoint("http://localhost:4566")
            .with_allow_http(true)
            .build()
            .unwrap();

        let object_path = object_store::path::Path::from("/course.parquet");
        let get_result = store
            .get(&object_path)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();

        let mut reader = ParquetRecordBatchReaderBuilder::try_new(get_result)
            .unwrap()
            .build()
            .unwrap();

        let actual_batch = reader.next().unwrap().unwrap();
        assert_eq!(actual_batch, expected_batch);
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

        let query = Query::new(
            "
                DROP TABLE IF EXISTS tempdb.dbo.canvas;
                CREATE TABLE tempdb.dbo.canvas (a int);
            ",
        );
        // let query = Query::new("INSERT INTO tempdb.dbo.canvas VALUES (234)");
        let results = query.execute(&mut client).await.unwrap();

        let plower = Plower::new("ms://sa:Seedy2025@localhost:1433").await;
        plower.seed("ms").await;

        let actual_val = client
            .simple_query("select a from tempdb.dbo.canvas")
            .await
            .unwrap()
            .into_row()
            .await
            .unwrap()
            .unwrap()
            .get::<i32, _>("a")
            .unwrap();

        assert_eq!(actual_val, 234);
    }

    #[tokio::test]
    async fn test_building_parquet() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let tmpdir = tmpdir.path().to_str().unwrap();
        let file_store = "file://".to_string() + tmpdir;
        let expected_batch = record_batch!(("a", Int32, [1, 2, 3])).unwrap();

        let plower = Plower::new(&file_store).await;
        // let recipe = "file { course.parquet <parquet> [ { topic: Chemistry } ] }";
        plower.seed("file").await;

        let filename = tmpdir.to_string().clone() + "/course.parquet";
        let file = std::fs::File::open(filename).unwrap();
        let mut reader = ParquetRecordBatchReaderBuilder::try_new(file)
            .unwrap()
            .build()
            .unwrap();

        let actual_batch = reader.next().unwrap().unwrap();
        println!("Record Batch {actual_batch:#?}");
        assert_eq!(actual_batch, expected_batch);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_seeding_a_ftp_server() {
        spin_up_ftp_server().await;

        let plower = Plower::new("ftp://user:testing@localhost").await;
        let recipe = "ftp { cars.parquet <parquet> [ { make: Honda } ] }";
        plower.seed("ftp").await;

        let mut ftp_stream = FtpStream::connect("127.0.0.1:2121").unwrap();

        ftp_stream.login("t", "k").unwrap();
        let contents = ftp_stream
            .list(None)
            .unwrap()
            .iter()
            .map(|f| list::File::try_from(f.as_str()).ok().unwrap())
            .map(|f| f.name().to_string())
            .collect::<Vec<_>>();

        let contents = contents.first().unwrap().as_str();

        assert_eq!(contents, "happens.txt");
    }

    #[tokio::test]
    async fn test_seeding_postgres() {
        let mut connection =
            PgConnection::connect("postgres://postgres:mysecretpassword@localhost/postgres")
                .await
                .unwrap();

        let _ = connection
            .execute(
                "
            DROP TABLE IF EXISTS show;
            CREATE TABlE show (name text);
        ",
            )
            .await
            .unwrap();

        let plower = Plower::new("postgres://postgres:mysecretpassword@localhost/postgres").await;
        plower.seed("postgres").await;

        let actual_value: (String,) = sqlx::query_as("Select name from show")
            .fetch_one(&mut connection)
            .await
            .unwrap();

        assert_eq!(actual_value.0, "tmnt");
    }

    #[tokio::test]
    async fn test_seeding_kafka() {
        let mut client_config = ClientConfig::new();
        client_config
            .set("bootstrap.servers", "localhost:9092")
            .set("group.id", "what")
            .set("enable.partition.eof", "false")
            .set("enable.auto.commit", "false");

        let admin_client = AdminClient::from_config(&client_config).unwrap();

        let _ = admin_client
            .delete_topics(&["garden"], &AdminOptions::new())
            .await
            .unwrap();

        let _ = admin_client
            .create_topics(
                vec![&NewTopic {
                    name: "garden",
                    num_partitions: 1,
                    config: vec![],
                    replication: TopicReplication::Fixed(1),
                }],
                &AdminOptions::new(),
            )
            .await
            .unwrap();

        let plower = Plower::new("kafka://localhost").await;
        plower.seed("kafka").await;

        let stream_consumer = client_config.create::<StreamConsumer>().unwrap();

        let mut tpl = TopicPartitionList::new();

        tpl.add_partition_offset("garden", 0, Offset::Beginning)
            .unwrap();

        stream_consumer.assign(&tpl).unwrap();

        while let Some(msg) = stream_consumer.stream().next().await {
            let Ok(actual_msg) = msg else { continue };
            let payload = actual_msg.payload().unwrap();
            let actual_msg = std::str::from_utf8(payload).unwrap();
            assert_eq!(actual_msg, "crabgrass");
            break;
        }
    }

    #[tokio::test]
    async fn test_sftp_server() {
        struct Client;

        impl client::Handler for Client {
            type Error = eyre::Report;

            async fn check_server_key(
                &mut self,
                server_public_key: &russh::keys::PublicKey,
            ) -> Result<bool, Self::Error> {
                Ok(true)
            }
        }

        let config = russh::client::Config::default();
        let sh = Client {};
        let mut session =
            russh::client::connect(std::sync::Arc::new(config), ("localhost", 22), sh)
                .await
                .unwrap();

        session.authenticate_password("foo", "pass").await.unwrap();

        let channel = session.channel_open_session().await.unwrap();
        channel.request_subsystem(true, "sftp").await.unwrap();
        let sftp = SftpSession::new(channel.into_stream()).await.unwrap();

        if sftp.try_exists("test/mystery").await.unwrap() {
            let _ = sftp.remove_file("test/mystery").await.unwrap();
        }

        let plower = Plower::new("sftp://localhost").await;
        plower.seed("sftp").await;

        let actual_filename = sftp
            .read_dir("test")
            .await
            .unwrap()
            .next()
            .unwrap()
            .file_name();

        assert_eq!(actual_filename, "mystery");
    }

    #[tokio::test]
    async fn test_seeding_redis() {
        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let mut connection = client.get_multiplexed_async_connection().await.unwrap();

        if connection.exists::<_, bool>("lawn").await.unwrap() {
            connection.del::<_, String>("lawn").await.unwrap();
        }

        let plower = Plower::new("redis://localhost").await;
        plower.seed("redis").await;

        let value = connection.get::<_, String>("lawn").await.unwrap();

        assert_eq!(value, String::from("turf"));
    }

    #[tokio::test]
    async fn test_seeding_mongodb() {
        let client = mongodb::Client::with_uri_str("mongodb://127.0.0.1/")
            .await
            .unwrap();

        let database = client.database("electronics");
        let collection: Collection<Document> = database.collection("tv");

        let doc = doc! { "brand": "LG" };
        let res = collection.delete_many(doc.clone()).await.unwrap();

        let plower = Plower::new("mongodb://localhost").await;
        plower.seed("mongodb").await;

        let mut l = collection.find(doc).await.unwrap();

        let document = l.next().await.unwrap().unwrap();

        let actual_value = document.get_str("brand").unwrap().to_string();

        assert_eq!(actual_value, String::from("LG"))
    }
}
