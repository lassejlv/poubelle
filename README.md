# Poubelle DB

A lightweight SQL database with minimal syntax, written in Rust.

## Architecture

- **storage**: Binary storage format with page-based architecture
- **parser**: Minimal SQL syntax parser
- **engine**: Query execution engine
- **server**: Combined TCP + HTTP server with username/password authentication
- **sdk**: Client libraries
  - **Rust**: `crates/sdk`
  - **TypeScript**: `sdk/typescript` (Bun)
  - **Go**: `sdk/go`

## Build

### Cargo

```bash
cargo build --release
```

### Docker

```bash
docker build -t poubelle .
```

## Benchmark

Run the TCP benchmark (requires server running):

```bash
./bench.sh
```

The script will:

- Connect via TCP
- Run 100 INSERT operations
- Run 10 SELECT queries
- Run 10 CREATE TABLE operations
- Display timing results

## Run

### Local

```bash
cargo run --bin poubelle
```

### Docker

```bash
docker run -p 5432:5432 -p 3000:3000 -v poubelle-data:/data poubelle
```

### Docker Compose

```bash
docker-compose up
```

Starts both servers:

- TCP server: `5432`
- HTTP API: `3000`

Default credentials: `admin` / `admin`

## Configuration

Configure via environment variables or `.env` file:

```bash
POUBELLE_DATA_DIR=./data         # Data storage directory
POUBELLE_HOST=127.0.0.1          # Server host
POUBELLE_PORT=5432               # Server port
POUBELLE_USERNAME=admin          # Authentication username
POUBELLE_PASSWORD=admin          # Authentication password

POUBELLE_HTTP_HOST=127.0.0.1    # HTTP gateway host
POUBELLE_HTTP_PORT=3000          # HTTP gateway port
```

Example:

```bash
cp .env.example .env
# Edit .env with your settings
cargo run --bin poubelle
```

## SQL Syntax

### CREATE TABLE

```sql
CREATE TABLE users (id INT, name TEXT)
```

### INSERT

```sql
INSERT INTO users (id, name) VALUES (1, 'Alice')
```

### SELECT

```sql
SELECT * FROM users
SELECT id, name FROM users
```

## Connect

### Connection String Format

```
poubelle://username:password@host:port
```

### TCP (telnet/netcat)

```bash
telnet localhost 5432
```

### HTTP (curl)

```bash
curl -X POST http://localhost:3000/query \
  -H "Content-Type: application/json" \
  -d '{"query": "SELECT * FROM users"}'
```

### Client SDKs

**TypeScript (Bun):**

```typescript
import PoubelleClient from 'poubelle-sdk'
const client = new PoubelleClient('poubelle://admin:admin@localhost:5432')
await client.connect()
```

**Go:**

```go
import poubelle "github.com/poubelle/sdk-go"
client, _ := poubelle.NewClient("poubelle://admin:admin@localhost:5432")
client.Connect()
```

**Rust:**

```rust
use poubelle_sdk::PoubelleClient;
let mut client = PoubelleClient::new("poubelle://admin:admin@localhost:5432")?;
client.connect().await?;
```
