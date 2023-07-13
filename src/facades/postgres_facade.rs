use std::env;
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, PoolBuilder};
use tokio_postgres::NoTls;

pub fn create_pool() -> Pool {
    let mut pg_config = tokio_postgres::Config::new();

    pg_config.host("127.0.0.1");
    pg_config.user("guest");
    pg_config.password("guest");
    pg_config.dbname("test");

    let mgr_config = ManagerConfig{recycling_method: RecyclingMethod::Fast};

    let mgr = Manager::from_config(pg_config, NoTls, mgr_config);
    
    Pool::builder(mgr).max_size(3).build().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn create_connection_pool() {
        let pool = create_pool();
    }

    #[tokio::test]
    async fn get_client() {
        let pool = create_pool();
        let client = pool.get().await.unwrap();
    }

    #[tokio::test]
    async fn test_statement() {
        let pool = create_pool();
        let client = pool.get().await.unwrap();

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