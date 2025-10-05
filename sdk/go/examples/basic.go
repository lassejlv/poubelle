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

	fmt.Println("Connecting to Poubelle DB...")
	if err := client.Connect(); err != nil {
		log.Fatal(err)
	}
	defer client.Close()
	fmt.Println("Connected!")

	fmt.Println("\nCreating table...")
	result, err := client.Query("CREATE TABLE users (id INT, name TEXT, age INT)")
	if err != nil {
		log.Fatal(err)
	}
	fmt.Println(result)

	fmt.Println("\nInserting data...")
	client.Query("INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30)")
	client.Query("INSERT INTO users (id, name, age) VALUES (2, 'Bob', 25)")
	client.Query("INSERT INTO users (id, name, age) VALUES (3, 'Charlie', 35)")
	fmt.Println("Inserted 3 rows")

	fmt.Println("\nQuerying data (debug format)...")
	rows, err := client.Execute("SELECT * FROM users")
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Found %d rows:\n", len(rows))
	for _, row := range rows {
		fmt.Printf("  %v\n", row)
	}

	fmt.Println("\nQuerying data (JSON format)...")
	jsonRows, err := client.ExecuteJSON("SELECT * FROM users WHERE age > 25")
	if err != nil {
		log.Fatal(err)
	}
	fmt.Printf("Found %d rows:\n", len(jsonRows))
	for _, row := range jsonRows {
		fmt.Printf("  %v\n", row)
	}

	fmt.Println("\nDone!")
}
