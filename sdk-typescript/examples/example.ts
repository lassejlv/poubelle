import PoubelleClient from '../src/index'

const client = new PoubelleClient('poubelle://admin:admin@127.0.0.1:5432')

await client.connect()

const users = await client.query('SELECT * FROM users')
console.log(users)

await client.close()
