# Poubelle TypeScript SDK

TypeScript SDK for connecting to Poubelle DB using Bun's TCP connector.

## Installation

```bash
cd sdk-typescript
bun install
```

## Usage

```typescript
import PoubelleClient from 'poubelle-sdk'

const client = new PoubelleClient('poubelle://admin:admin@127.0.0.1:5432')

await client.connect()

await client.createTable('users', {
  id: 'INT',
  name: 'TEXT',
})

await client.insert('users', { id: 1, name: 'Alice' })

const rows = await client.select('users')
console.log(rows)

await client.close()
```

## API

### `new PoubelleClient(connectionString)`

Create a new client instance.

**Connection String Format:**

```
poubelle://username:password@host:port
```

**Example:**

```typescript
const client = new PoubelleClient('poubelle://admin:admin@localhost:5432')
```

### `connect()`

Connect to the database and authenticate.

### `query(sql: string)`

Execute a raw SQL query.

### `createTable(name: string, columns: Record<string, "INT" | "TEXT">)`

Create a new table.

### `insert(table: string, data: Record<string, number | string | null>)`

Insert a row into a table.

### `select(table: string, columns?: string[])`

Select rows from a table. Returns parsed rows as objects.

### `close()`

Close the connection.

## Example

```bash
bun run example
```
