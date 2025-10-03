# Poubelle DB

A lightweight SQL database with minimal syntax, written in Rust.

## Architecture

- **storage-engine**: Binary storage format with page-based architecture
- **parser**: Minimal SQL syntax parser
- **db-engine**: Query execution engine
- **server**: TCP server with username/password authentication

## Build

```bash
cargo build --release
```

## Run

```bash
cargo run --bin poubelle
```

Server listens on `127.0.0.1:5432`

Default credentials: `admin` / `admin`

## Configuration

Configure via environment variables or `.env` file:

```bash
POUBELLE_DATA_DIR=./data         # Data storage directory
POUBELLE_HOST=127.0.0.1          # Server host
POUBELLE_PORT=5432               # Server port
POUBELLE_USERNAME=admin          # Authentication username
POUBELLE_PASSWORD=admin          # Authentication password
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

```bash
telnet localhost 5432
```
