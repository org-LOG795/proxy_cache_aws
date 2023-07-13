use std::env;
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use tokio_postgres::{Config, NoTls};

pub fn create_config_from_env() -> Result<Config, String> {
    match (env::var("POSTGRES_HOST"),
           env::var("POSTGRES_USER"),
           env::var("POSTGRES_PASSWORD"),
           env::var("POSTGRES_DB")) 
    {
        (Ok(host), Ok(user), Ok(password), Ok(db)) => {
            let mut config = Config::new();
            config.host(&host);
            config.user(&user);
            config.password(&password);
            config.dbname(&db);
            Ok(config)
        },
        _ => Err("Missing postgres env".to_string())
    }
}

pub fn create_pool(configs: Config) -> Result<Pool, String> {
    let mgr_config = ManagerConfig{recycling_method: RecyclingMethod::Fast};
    let mgr = Manager::from_config(configs, NoTls, mgr_config);
    let pool_result = Pool::builder(mgr).max_size(3).build();
    match pool_result {
        Ok(pool) => Ok(pool),
        Err(err) => Err(err.to_string()),
    }
    
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn setup_env() {
        env::set_var("POSTGRES_HOST", "127.0.0.1");
        env::set_var("POSTGRES_USER", "guest");
        env::set_var("POSTGRES_PASSWORD", "guest");
        env::set_var("POSTGRES_DB", "test");
    }

    #[test]
    fn err_if_no_env() {
        let config = create_config_from_env();
        assert!(config.is_err());
    }

    #[test]
    fn create_connection_pool() {
        setup_env();
        let pg_config = create_config_from_env().unwrap();
        let pool = create_pool(pg_config);
        assert!(pool.is_ok());
    }

    #[tokio::test]
    async fn create_client() {
        setup_env();
        let pg_config = create_config_from_env().unwrap();
        let pool = create_pool(pg_config);
        assert!(pool.is_ok());
        let client = pool.unwrap().get().await;
        assert!(client.is_ok())
    }

    #[tokio::test]
    async fn select_statement() {
        setup_env();
        let pg_config = create_config_from_env().unwrap();
        let pool = create_pool(pg_config);
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