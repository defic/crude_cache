use std::collections::HashMap;
use tokio::sync::{RwLock, RwLockWriteGuard, RwLockReadGuard};
use std::sync::Arc;
use std::hash::{Hash, Hasher};

#[derive(Default)]
pub struct ShardedMap<V: Send + Sync + 'static> {
    shards: Vec<Arc<RwLock<HashMap<String, V>>>>,
    num_shards: usize,
}

impl<V: Send + Sync> ShardedMap<V> {
    pub fn new(num_shards: usize) -> Self {
        let shards = (0..num_shards).map(|_| Arc::new(RwLock::new(HashMap::new()))).collect();

        ShardedMap { shards, num_shards }
    }

    fn get_shard(&self, key: &str) -> &Arc<RwLock<HashMap<String, V>>> {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hash);
        let index = (hash.finish() % self.num_shards as u64) as usize;
        &self.shards[index]
    }

    pub async fn get_write_lock(&self, key: &str) -> RwLockWriteGuard<'_, HashMap<String, V>> {
        let shard = self.get_shard(key);
        shard.write().await
    }

    pub async fn get_read_lock(&self, key: &str) -> RwLockReadGuard<'_, HashMap<String, V>> {
        let shard = self.get_shard(key);
        shard.read().await
    }

    pub async fn insert(&self, key: String, value: V) {
        let shard = self.get_shard(&key);
        let mut map = shard.write().await;
        map.insert(key, value);
    }
}