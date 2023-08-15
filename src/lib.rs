#![deny(clippy::all)]

//! # CrudeCache
//!
//! A simple utility for good enough concurrent caching needs.
//!
//! ## Overview
//!
//! `CrudeCache` uses a sharding approach to improve concurrent access. It divides the cached data across multiple internal `HashMap`s to reduce contention. Each shard is protected by its own `tokio::sync::RwLock`, ensuring safe concurrent read and write operations.
//!
//! For an example usage in application dataservice, check in [tests/cache_test.rs](https://github.com/defic/crude_cache/blob/master/tests/cache_test.rs).
//! 
//! ## Basic Usage
//!
//! ```rust
//! use std::time::Duration;
//! use crude_cache::CrudeCache;
//!
//! #[tokio::test]
//! async fn example_test() {
//!     let get_slow_data = || async {
//!         tokio::time::sleep(Duration::from_secs(5)).await;
//!         "test data"
//!     };
//!
//!     let cache = CrudeCache::default();
//!     //takes 5secs
//!     let big_data = cache.get_or_else_update("cachekey", Duration::from_secs(60), get_slow_data).await;
//!     //gets the cached data
//!     let big_data = cache.get_or_else_update("cachekey", Duration::from_secs(60), get_slow_data).await;
//! }
//! ```
//!
//! ## Disclaimer
//!
//! Please note that `CrudeCache` was developed primarily for personal projects and has not been battle-tested in large-scale production environments. Contributions, suggestions, and feedback are always welcome!
//!

mod sharded_map;

use sharded_map::ShardedMap;
use std::time::{Duration, Instant};
use std::{sync::Arc, any::Any};
use std::future::Future;

type Item = (Instant, Box<dyn Any + Sync + Send>);

#[derive(Clone)]
pub struct CrudeCache {
    inner: Arc<ShardedMap<String, Item>>
}

impl Default for CrudeCache {
    fn default() -> Self {
        CrudeCache {
            inner: Arc::new(ShardedMap::new(64)),
        }
    }
}

impl CrudeCache {

    /// * `num_shards` - Determines how many internal Shards (HashMaps) cache has. `default()` uses 64.
    pub fn new(num_shards: usize) -> Self {
        CrudeCache {
            inner: Arc::new(ShardedMap::new(num_shards)),
        }
    }

    /// Inserts new value regardless of existing items expiration.
    /// Causes writelock for the shard
    pub async fn force_insert<V>(&self, k: impl Into<String>, v: V, duration: Duration)
    where
        V: 'static + Sync + Send + Clone,
    {
        self.inner.insert(k.into(), (Instant::now() + duration, Box::new(v))).await;
    }

    async fn update<V, F, Fut>(&self, k: impl Into<String>, expiration: Duration, get: F) -> V 
    where 
        V: 'static + Clone + Sync + Send,
        F: Fn() -> Fut,
        Fut: Future<Output = V>
    {
        let k = k.into();
        //write locks item
        let mut write = self.inner.get_write_lock(&k).await;
        
        match write.get(&k) {
            Some((instant, value)) if instant > &Instant::now() => {
                //no inserting, return cached value
                let value = value.downcast_ref::<V>();
                assert!(value.is_some(), "CrudeCache update: keys '{}' value is not type: {:?}", k, std::any::type_name::<V>());
                let value = value.cloned();
                value.unwrap()
            },
            Some(_) | None => {
                let value = get().await;
                write.insert(k, (Instant::now() + expiration, Box::new(value.clone())));
                value
            },
        }

    }

    /// Gets the item by its key. If the value does not exist or it has expired,
    /// returns None.
    ///
    /// # Panics
    ///
    /// Panics if value of the cached type is not `V`.
    pub async fn get<V: 'static + Clone>(&self, key: &str) -> Option<V> {
        let read = self.inner.get_read_lock(key).await;
        let item = read.get(key);
        match item {
            Some((instant, item)) if instant > &Instant::now() => {
                let item = item.downcast_ref::<V>();
                assert!(item.is_some(), "CrudeCache get: keys '{}' value is not type: {:?}", key, std::any::type_name::<V>());
                item.cloned()
            },
            Some (_) | None => None
        }
    }

    /// Gets the item by its key. If the value does not exist or it has expired,
    /// new value by calling the provided `get` Fn.
    ///
    /// # Panics
    ///
    /// Causes panic if value of the cached type is not `V`.
    pub async fn get_or_else_update<V, F, Fut>(&self, key: &str, expiration: Duration, get: F) -> V
    where 
        V: 'static + Clone + Sync + Send,
        F: Fn() -> Fut,
        Fut: Future<Output = V>
    {   
        let cached = self.get::<V>(key).await;
        match cached {
            None => {
                self.update(key, expiration, get).await
            },
            Some(v) => v,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[should_panic( expected = "CrudeCache get: keys 'h' value is not type: \"alloc::vec::Vec<&str>\"")]
    async fn wrong_type_panic() {
        let tt = || async {
            vec![5,4,3,2,1,0]
        };
    
        let cache = CrudeCache::default();
        cache.get_or_else_update("h", Duration::from_secs(2), tt).await;
        cache.get_or_else_update("h", Duration::from_secs(2), async_string_vec).await;
    }
    
    async fn async_string_vec() -> Vec<&'static str> {
        tokio::time::sleep(Duration::from_millis(100)).await;
        vec!["dp", "op", "kp"]
    }
    
    #[tokio::test]
    async fn expiration_test() {
        let cache = CrudeCache::default();
        let key = "h";
        let val = cache.get_or_else_update(key, Duration::from_millis(500), || async {"testi"}).await;
        assert_eq!("testi", val);
    
        tokio::time::sleep(Duration::from_millis(200)).await;
        let val: Option<&str> = cache.get(key).await;
        assert_eq!(Some("testi"), val);
    
        tokio::time::sleep(Duration::from_millis(301)).await;
        let val: Option<&str> = cache.get(key).await;
        assert_eq!(None, val);
    }
    
    #[tokio::test]
    async fn update_test() {
        let cache = CrudeCache::default();
        let key = "h";
        let val = cache.get_or_else_update(key, Duration::from_millis(200), async_usize_vec).await;
        assert_eq!(vec![5,4,3,2,1,0], val);

        tokio::time::sleep(Duration::from_millis(201)).await;
        let val = cache.get_or_else_update(key, Duration::from_millis(500), async_usize_vec2).await;
        assert_eq!(vec![5,4,3,2,1,1], val);
    }
    
    async fn async_usize_vec() -> Vec<usize> {
        tokio::time::sleep(Duration::from_millis(2)).await;
        vec![5,4,3,2,1,0]
    }
    
    async fn async_usize_vec2() -> Vec<usize> {
        tokio::time::sleep(Duration::from_millis(2)).await;
        vec![5,4,3,2,1,1]
    }
}