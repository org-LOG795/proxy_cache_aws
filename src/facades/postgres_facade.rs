use std::env;
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, ClientWrapper, Object};
use tokio_postgres::{Config, NoTls};
use chrono::{Utc, Datelike};
use tracing_subscriber::fmt::format;

pub fn create_config(host: &str, user: &str, pass: &str, db: &str) -> Config {
    let mut configs = Config::new();

    configs.host(host);
    configs.user(user);
    configs.password(pass);
    configs.dbname(db);

    configs
}
pub fn create_config_from_env() -> Result<Config, String> {
    match (env::var("POSTGRES_HOST"),
           env::var("POSTGRES_USER"),
           env::var("POSTGRES_PASSWORD"),
           env::var("POSTGRES_DB")) 
    {
        (Ok(host), Ok(user), Ok(password), Ok(db)) 
            => Ok(create_config(&host, &user, &password, &db)),
        _ => Err("Missing postgres env".to_string())
    }
}

pub fn create_pool(configs: Config, pool_size: usize) -> Result<Pool, String> {
    let mgr_config = ManagerConfig{recycling_method: RecyclingMethod::Fast};
    let mgr = Manager::from_config(configs, NoTls, mgr_config);
    let pool_result = Pool::builder(mgr).max_size(pool_size).build();
    pool_result.map_err(|e| e.to_string())
}

fn get_current_date() -> String {
    let current_date = Utc::now();
    format!("{:04}-{:02}-{:02}", current_date.year(), current_date.month(), current_date.day())
}

pub async fn get_offset(client: Object, len_bytes: usize) -> Result<(i64, i64), String>{
    let current_date = get_current_date();
    let table = "\"CacheOffsetTable\"".to_string();
    let date = "\"date\"".to_string();
    let offset = "\"offset\"".to_string();
    
    let query = format!(
        "INSERT INTO public.{table} ({date}, {offset}) VALUES (DATE '{current_date}', {len_bytes})\n
        ON CONFLICT ({date}) DO\n
        UPDATE SET {offset} = {table}.{offset} + {len_bytes}\n
        RETURNING {table}.{offset};\n", 
        table = table, date = date, offset = offset, len_bytes = len_bytes);

    match client.prepare_cached(&query).await {
        Ok(statement) => {
            client
                .query_one(&statement, &[])
                .await
                .map(|row| {
                    let offset: i64 = row.get("offset");
                    (offset - len_bytes as i64, offset - 1)
                })
                .map_err(|err| err.to_string())
        },
        Err(err) => Err(err.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_config() -> Config { create_config("127.0.0.1", "guest", "guest", "test") }
    
    #[test]
    //a added to the test name to make sure this test is ran first
    fn err_if_no_env() {
        let config = create_config_from_env();
        assert!(config.is_err());
    }

    #[test]
    fn create_connection_pool() {
        let pg_config = get_test_config();
        let pool = create_pool(pg_config, 1);
        assert!(pool.is_ok());
    }

    #[tokio::test]
    #[ignore = "tests need to be ran with local postgres db"]
    async fn create_client() {
        let pg_config = get_test_config();
        let pool = create_pool(pg_config, 1);
        assert!(pool.is_ok());
        let client = pool.unwrap().get().await;
        assert!(client.is_ok())
    }

    #[tokio::test]
    #[ignore = "tests need to be ran with local postgres db"]
    async fn select_statement() {
        let pg_config = get_test_config();
        let pool = create_pool(pg_config, 1);
        assert!(pool.is_ok());
        let client = pool.unwrap().get().await.unwrap();

        let statement = client.prepare_cached("SELECT * FROM public.table_test").await.unwrap();
        let rows = client.query(&statement, &[]).await.unwrap();

        println!("------------");
        for row in rows.iter() {
            for (index, column) in row.columns().iter().enumerate() {
                let value : i32 = row.get(index);
                println!("{}: {:?}", column.name(), value);
            }
        }
        println!("------------");
    }

    #[tokio::test]
    #[ignore = "tests need to be ran with local postgres db"]
    async fn get_offset_test() {
        let pg_config = get_test_config();
        let pool = create_pool(pg_config, 1);
        assert!(pool.is_ok());
        let client = pool.unwrap().get().await.unwrap();

        match get_offset(client, 20).await {
            Ok(offsets) => println!("({},{})", offsets.0, offsets.1),
            Err(err) => println!("{}", err.to_string())
        }
    }
}