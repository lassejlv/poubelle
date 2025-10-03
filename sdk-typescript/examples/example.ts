import PoubelleClient from '../src/index'
const client = new PoubelleClient({
  host: '127.0.0.1',
  port: 5432,
  username: 'admin',
  password: 'admin',
})

await client.connect()

const users = await client.query('SELECT * FROM users')
console.log(users)

await client.close()
