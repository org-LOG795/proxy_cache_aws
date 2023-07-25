async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

async fn say_secret(State(config) : State<Config>) -> String {
    return config.secret;
}

#[derive(Serialize, Deserialize)]
struct TestRecord {
    // Define the fields based on your table schema
    column1: i32,
}

async fn postgres_test_handler(State(pool_manager) : State<Pool>) -> Json<Vec<TestRecord>>{
    let client = pool_manager.get().await.unwrap();

    let statement = client.prepare_cached("SELECT * FROM public.test_table").await.unwrap();

    let rows = client.query(&statement, &[]).await.unwrap();

    let mut records = Vec::new();

    for row in rows.iter() {
        let mut record = TestRecord {
            column1: 0, // Initialize with default values
            // ...
        };

        for (index, column) in row.columns().iter().enumerate() {
            match column.name() {
                "column1" => record.column1 = row.get(index),
                // Set other fields based on their column names
                // ...
                _ => (),
            }
        }

        records.push(record)
    }

    Json(records)
}