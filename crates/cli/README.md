# Poubelle CLI

A psql-like interactive CLI for Poubelle DB.

## Features

- **Interactive REPL**: Command-line interface with readline support
- **Command History**: Persistent history saved to `~/.poubelle_history`
- **Multi-line SQL**: Support for multi-line SQL statements (end with `;`)
- **Pretty Tables**: Formatted table output for query results
- **Query Timing**: Displays execution time for each query
- **Meta-commands**: psql-like commands for common operations

## Installation

Build the CLI:

```bash
cargo build --release --bin poubelle-cli
```

The binary will be available at `target/release/poubelle-cli`.

## Usage

### Interactive Mode

Start the interactive shell:

```bash
./target/release/poubelle-cli
```

With custom connection:

```bash
./target/release/poubelle-cli -c "poubelle://admin:admin@localhost:5432"
```

### Single Command Mode

Execute a single command and exit:

```bash
./target/release/poubelle-cli -e "SELECT * FROM users;"
```

## Meta-commands

| Command             | Description          |
| ------------------- | -------------------- |
| `\q`, `\quit`       | Exit the CLI         |
| `\?`, `\h`, `\help` | Show help message    |
| `\l`, `\list`       | List all tables      |
| `\dt`               | List all tables      |
| `\c`                | Show connection info |
| `Ctrl+C`            | Cancel current input |
| `Ctrl+D`            | Exit (same as `\q`)  |

## Examples

### Interactive Session

```
$ ./target/release/poubelle-cli
Connecting to Poubelle DB...
Connected to Poubelle DB
Type 'help' for help, '\q' to quit

poubelle> CREATE TABLE users (id INT, name TEXT, age INT);
OK (2ms)

poubelle> INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30);
OK (1ms)

poubelle> INSERT INTO users (id, name, age) VALUES (2, 'Bob', 25);
OK (0ms)

poubelle> SELECT * FROM users;
+----+-------+-----+
| id | name  | age |
+----+-------+-----+
| 1  | Alice | 30  |
+----+-------+-----+
| 2  | Bob   | 25  |
+----+-------+-----+

2 row(s) (1ms)

poubelle> \q
Goodbye!
```

### Multi-line SQL

```
poubelle> CREATE TABLE products (
       ->   id INT,
       ->   name TEXT,
       ->   price INT
       -> );
OK (3ms)
```

### Command-line Execution

```bash
# Create table
./target/release/poubelle-cli -e "CREATE TABLE logs (id INT, message TEXT);"

# Insert data
./target/release/poubelle-cli -e "INSERT INTO logs (id, message) VALUES (1, 'System started');"

# Query data
./target/release/poubelle-cli -e "SELECT * FROM logs;"
```

## Command-line Options

```
Usage: poubelle-cli [OPTIONS]

Options:
  -c, --connection <CONNECTION>  Connection string [default: poubelle://admin:admin@127.0.0.1:5432]
  -e, --command <COMMAND>        Execute a single command and exit
  -h, --help                     Print help
```

## Connection String Format

```
poubelle://username:password@host:port
```

Examples:

- `poubelle://admin:admin@localhost:5432`
- `poubelle://user:pass@192.168.1.100:5432`
- `poubelle://admin:admin@127.0.0.1:5432`
