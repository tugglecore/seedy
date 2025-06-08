use tokio::sync::Mutex;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};
use duckdb::Result;
use crate::recipe::*;
use async_trait::async_trait;
use crate::Store;

pub struct DuckStore {
    query_sender: Sender<Order>,
    row_receiver: Mutex<Receiver<String>>,
    handle: std::thread::JoinHandle<()>,
}

fn seed_order(connection: &duckdb::Connection) {
    let result = connection.execute("select 1", []);
}

impl DuckStore {
    pub fn from_connection(connection: duckdb::Connection) -> Self {
        let (query_sender, mut query_receiver) = channel(100);
        let (row_sender, row_receiver) = channel(100);
        let row_receiver = Mutex::new(row_receiver);
        let handle = std::thread::spawn(move || {
            while let Some(query) = query_receiver.blocking_recv() {
                let result = connection.execute("INSERT INTO a VALUES (1);", []);

                seed_order(&connection);
                row_sender.blocking_send(String::new());
            }
        });

        Self {
            handle,
            row_receiver,
            query_sender,
        }
    }


    fn fulfill_order(order: Order) {
        let table_name = order.store_location.last().unwrap().clone();


    }


    pub fn new() -> Self {
        let connection = duckdb::Connection::open_in_memory().unwrap();
        DuckStore::from_connection(connection)
    }
}

#[async_trait]
impl Store for DuckStore {
    async fn plant(&self, order: &Order) {
        self.query_sender.send(order.clone()).await.unwrap();
        let mut receiver = self.row_receiver.lock().await;
        receiver.recv().await;
    }
}
