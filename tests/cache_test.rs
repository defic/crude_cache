#[macro_use]
extern crate timeit;
use std::{time::Duration, sync::Arc};
use crude_cache::CrudeCache;
use futures::future::join_all;


#[derive(Default, Clone)]
pub struct Db {}
impl Db {
    pub async fn big_data(&self) -> Vec<usize> {
        println!("-- DB -- big data called");
        tokio::time::sleep(Duration::from_secs(1)).await;
        vec![9,8,7,6,5,4,3,2,1,0]
    }
}

#[derive(Default, Clone)]
pub struct DataService {
    cache: Arc<CrudeCache>,
    db: Arc<Db>
}

impl DataService {
    pub async fn get_big_data(&self) -> Vec<usize> {
        self.cache.get_or_else_update("get_big_data", Duration::from_secs(2), || self.db.big_data()).await
    }
}

#[tokio::test]
async fn multi_access() {
    
    let ds = DataService::default();

    let d = timeit_loops!(1, {
        ds.get_big_data().await;
    });
    println!("Seconds: {d}");

    let d = timeit_loops!(1, {
        ds.get_big_data().await;
    });
    println!("Seconds: {d}");

    let d = timeit_loops!(1, {
        ds.get_big_data().await;
    });
    println!("Seconds: {d}");
    
    let d = timeit_loops!(1, {
        let join_handles: Vec<_> = (0..100).map(|_| {
            let ds = ds.clone();
            tokio::spawn(async move {
                ds.get_big_data().await;
            })
        }).collect();

        join_all(join_handles).await;
    });
    println!("Seconds: {d}");
    assert!(d < 0.1f64);

    tokio::time::sleep(Duration::from_secs(2)).await;

    let d = timeit_loops!(1, {
        let join_handles: Vec<_> = (0..100).map(|_| {
            let ds = ds.clone();
            tokio::spawn(async move {
                ds.get_big_data().await;
            })
        }).collect();

        join_all(join_handles).await;
    });
    println!("Seconds: {d}");
    assert!(d > 1f64 && d < 1.1f64)
    
}

#[tokio::test]
async fn shard_test() {
    let cache = CrudeCache::default();

    let cache1 = cache.clone();
    let jh1 = tokio::spawn(async move {
        cache1.get_or_else_update("0000", Duration::from_secs(60), slow_async).await;
    });

    let cache2 = cache.clone();
    let jh2 = tokio::spawn(async move {
        cache2.get_or_else_update("yxyxyxyxyx", Duration::from_secs(60), slow_async).await;
    });


    let cache3 = cache.clone();
    let jh3 = tokio::spawn(async move {
        cache3.get_or_else_update("0000", Duration::from_secs(60), slow_async).await;
    });

    join_all(vec![jh1,jh2, jh3]).await;
}


async fn slow_async() -> String {
    tokio::time::sleep(Duration::from_secs(5)).await;
    "Very slow".to_string()
}