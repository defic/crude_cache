
use crate::sharded_map::ShardedMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::future::Future;

#[derive(Clone)]
pub struct Item<V: 'static + Sync + Send + Clone>
{
    pub instant: Instant,
    pub value: V,
}

#[derive(Clone)]
pub struct CrudeCache {
    inner: Arc<ShardedMap<String>>
}

impl Default for CrudeCache {
    fn default() -> Self {
        CrudeCache {
            inner: Arc::new(ShardedMap::new(64)),
        }
    }
}

impl CrudeCache {
    /// * `num_shards` - Determines how many internal Shards (`HashMaps`) cache has. `default()` uses 64.
    pub fn new(num_shards: usize) -> Self {
        CrudeCache {
            inner: Arc::new(ShardedMap::new(num_shards)),
        }
    }
    
    /// Inserts new value regardless of existing items expiration.
    /// Causes writelock for the shard
    pub async fn force_insert<V>(&self, key: String, value: V, expiration: Duration)
    where
        V: 'static + Sync + Send + Clone,
    {
        let item = Item {instant: Instant::now() + expiration, value};
        self.inner.insert(key, item).await;
    }
    
    /// locks the whole shard for the duration of the async endpoint
    async fn update_atomic<V, F, Fut>(&self, key: String, expiration: Duration, get: F) -> V 
    where 
        V: 'static + Clone + Sync + Send,
        F: Fn() -> Fut,
        Fut: Future<Output = V>
    {
        let lock = self.inner.get_write_lock(&key).await;
        let value = get().await;
        let item = Item {instant: Instant::now() + expiration, value: value.clone()};
        ShardedMap::insert_with_lock(item, key, lock);
        value
    }

    /// non-atomic: fist calls async function, then lock cache and update. Causes lot of endpoint calls, since locking is done after result.
    async fn update<V, F, Fut>(&self, key: &str, expiration: Duration, get: F) -> V 
    where 
        V: 'static + Clone + Sync + Send,
        F: Fn() -> Fut,
        Fut: Future<Output = V>
    {
        let value = get().await;
        let item: Option<Item<V>> = self.inner.get(key).await;
        match item {
            Some(Item{instant, value}) if instant > Instant::now() => value,
            Some(_) | None => {
                self.force_insert(key.into(), value.clone(), expiration).await;
                value
            }
        }
    }

    /// Gets the item by its key. If the value does not exist or it has expired,
    /// returns None.
    ///
    /// # Panics
    ///
    /// Panics if value of the cached type is not `V`.
    pub async fn get<V: 'static + Clone + Send + Sync>(&self, key: &str) -> Option<V> {
        let item: Item<V> = self.inner.get(key).await?;
        if item.instant < Instant::now() {
            return None;
        }
        Some(item.value)
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
                self.update_atomic(key.into(), expiration, get).await
            },
            Some(v) => v,
        }
    }

    pub async fn remove(&self, key: &str) -> bool {
        self.inner.remove(key).await
    }
}



#[cfg(test)]
mod tests {
    use crate::CrudeCache;
    use std::time::Duration;

    #[tokio::test]
    #[should_panic( expected = "CrudeCache get: keys 'h' value is not type: \"crude_cache::crude_cache::Item<alloc::vec::Vec<&str>>\"")]
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
