mod auth;
mod handler;

use auth::AuthStore;
use db_engine::Engine;
use handler::handle_client;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use storage_engine::Storage;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let data_dir = env::var("POUBELLE_DATA_DIR").unwrap_or_else(|_| "./data".to_string());
    let host = env::var("POUBELLE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("POUBELLE_PORT").unwrap_or_else(|_| "5432".to_string());
    let username = env::var("POUBELLE_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let password = env::var("POUBELLE_PASSWORD").unwrap_or_else(|_| "admin".to_string());

    let storage = Storage::open(PathBuf::from(&data_dir))?;
    let engine = Arc::new(Mutex::new(Engine::new(storage)));

    let auth_store = Arc::new(Mutex::new(AuthStore::new()));
    auth_store.lock().await.add_user(username, password);

    let bind_addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&bind_addr).await?;
    println!("Poubelle DB listening on {}", bind_addr);

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("Connection from: {}", addr);

        let engine = Arc::clone(&engine);
        let auth = Arc::clone(&auth_store);

        tokio::spawn(async move {
            let (reader, mut writer) = socket.into_split();
            let mut reader = BufReader::new(reader);

            writer.write_all(b"Username: ").await.ok();
            writer.flush().await.ok();

            let mut username = String::new();
            if reader.read_line(&mut username).await.is_err() {
                return;
            }
            username = username.trim().to_string();

            writer.write_all(b"Password: ").await.ok();
            writer.flush().await.ok();

            let mut password = String::new();
            if reader.read_line(&mut password).await.is_err() {
                return;
            }
            password = password.trim().to_string();

            let authenticated = auth.lock().await.verify(&username, &password);

            if !authenticated {
                writer.write_all(b"Authentication failed\n").await.ok();
                return;
            }

            writer.write_all(b"Connected to Poubelle DB\n").await.ok();

            handle_client(reader, writer, engine).await;
        });
    }
}
