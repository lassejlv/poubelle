use db_engine::{Engine, QueryResult};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex;

pub async fn handle_client(
    mut reader: BufReader<OwnedReadHalf>,
    mut writer: OwnedWriteHalf,
    engine: Arc<Mutex<Engine>>,
) {
    loop {
        writer.write_all(b"poubelle> ").await.ok();
        writer.flush().await.ok();

        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }

        let query = line.trim();
        if query.is_empty() {
            continue;
        }

        if query.eq_ignore_ascii_case("exit") || query.eq_ignore_ascii_case("quit") {
            writer.write_all(b"Goodbye\n").await.ok();
            break;
        }

        let mut engine = engine.lock().await;
        match engine.execute_query(query) {
            Ok(result) => {
                let output = format_result(result);
                writer.write_all(output.as_bytes()).await.ok();
            }
            Err(e) => {
                let msg = format!("Error: {}\n", e);
                writer.write_all(msg.as_bytes()).await.ok();
            }
        }
    }
}

fn format_result(result: QueryResult) -> String {
    match result {
        QueryResult::Success(msg) => format!("{}\n", msg),
        QueryResult::Rows(rows) => {
            if rows.is_empty() {
                return "No rows\n".to_string();
            }

            let mut output = String::new();
            for row in rows {
                output.push_str(&format!("{:?}\n", row.data));
            }
            output
        }
    }
}
