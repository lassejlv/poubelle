# Poubelle Rust SDK

Rust client library for Poubelle DB.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
poubelle-sdk = { path = "path/to/poubelle/crates/sdk" }
tokio = { version = "1", features = ["full"] }
```

## Usage

```rust
use poubelle_sdk::PoubelleClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = PoubelleClient::new("poubelle://admin:admin@127.0.0.1:5432")?;

    client.connect().await?;

    client.query("CREATE TABLE users (id INT, name TEXT)").await?;

    client.query("INSERT INTO users (id, name) VALUES (1, 'Alice')").await?;

    let rows = client.execute("SELECT * FROM users").await?;
    for row in rows {
        println!("{:?}", row);
    }

    client.close().await?;

    Ok(())
}
```

## API

### `PoubelleClient::new(connection_string: &str)`

Create a new client with a connection string.

**Format:** `poubelle://username:password@host:port`

### `connect() -> Result<()>`

Connect to the database and authenticate.

### `query(sql: &str) -> Result<String>`

Execute a SQL query and return the raw result string.

### `execute(sql: &str) -> Result<Vec<Row>>`

Execute a SELECT query and return parsed rows.

### `close() -> Result<()>`

Close the connection.

## Example

Run the example:

```bash
cargo run --example basic
```
