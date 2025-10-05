# Poubelle Go SDK

Go client library for Poubelle DB.

## Installation

```bash
go get github.com/poubelle/sdk-go
```

## Usage

```go
package main

import (
    "fmt"
    "log"

    poubelle "github.com/poubelle/sdk-go"
)

func main() {
    client, err := poubelle.NewClient("poubelle://admin:admin@127.0.0.1:5432")
    if err != nil {
        log.Fatal(err)
    }

    if err := client.Connect(); err != nil {
        log.Fatal(err)
    }
    defer client.Close()

    // Create table
    client.Query("CREATE TABLE users (id INT, name TEXT)")

    // Insert data
    client.Query("INSERT INTO users (id, name) VALUES (1, 'Alice')")

    // Query with debug format
    rows, err := client.Execute("SELECT * FROM users")
    if err != nil {
        log.Fatal(err)
    }
    for _, row := range rows {
        fmt.Printf("%v\n", row)
    }

    // Query with JSON format
    jsonRows, err := client.ExecuteJSON("SELECT * FROM users")
    if err != nil {
        log.Fatal(err)
    }
    for _, row := range jsonRows {
        fmt.Printf("ID: %v, Name: %v\n", row["id"], row["name"])
    }
}
```

## API

### `NewClient(connectionString string) (*Client, error)`

Create a new client with a connection string.

**Format:** `poubelle://username:password@host:port`

### `Connect() error`

Connect to the database and authenticate.

### `Query(sql string) (string, error)`

Execute a SQL query and return the raw result string.

### `Execute(sql string) ([]Row, error)`

Execute a query and return parsed rows (debug format).

### `ExecuteJSON(sql string) ([]Row, error)`

Execute a query with JSON format and return parsed rows.

### `Close() error`

Close the connection.

## Example

Run the example:

```bash
cd examples
go run basic.go
```
