use std::env;
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::{Config, NoTls};

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
}