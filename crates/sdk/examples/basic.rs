use poubelle_sdk::PoubelleClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = PoubelleClient::new("poubelle://admin:admin@127.0.0.1:5432")?;

    println!("Connecting to Poubelle DB...");
    client.connect().await?;
    println!("Connected!");

    println!("\nCreating table...");
    let result = client
        .query("CREATE TABLE users (id INT, name TEXT, age INT)")
        .await?;
    println!("{}", result);

    println!("\nInserting data...");
    client
        .query("INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30)")
        .await?;
    client
        .query("INSERT INTO users (id, name, age) VALUES (2, 'Bob', 25)")
        .await?;
    client
        .query("INSERT INTO users (id, name, age) VALUES (3, 'Charlie', 35)")
        .await?;
    println!("Inserted 3 rows");

    println!("\nQuerying data...");
    let rows = client.execute("SELECT * FROM users").await?;
    println!("Found {} rows:", rows.len());
    for row in rows {
        println!("  {:?}", row);
    }

    println!("\nClosing connection...");
    client.close().await?;
    println!("Done!");

    Ok(())
}
