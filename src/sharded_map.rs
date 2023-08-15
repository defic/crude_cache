use std::borrow::Borrow;
use std::collections::HashMap;
use tokio::sync::{RwLock, RwLockWriteGuard, RwLockReadGuard};
use std::sync::Arc;
use std::hash::{Hash, Hasher};

#[derive(Default)]
pub struct ShardedMap<K, V> {
    shards: Vec<Arc<RwLock<HashMap<K, V>>>>,
    num_shards: usize,
}

impl<K: Hash + Eq, V: Send + Sync> ShardedMap<K, V> {
    pub fn new(num_shards: usize) -> Self {
        let shards = (0..num_shards).map(|_| Arc::new(RwLock::new(HashMap::new()))).collect();

        ShardedMap { shards, num_shards }
    }

    fn get_shard<BK>(&self, key: &BK) -> &Arc<RwLock<HashMap<K, V>>> 
    where
        BK: Hash + Eq + ?Sized,
        K: Borrow<BK>
    {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hash);
        let index = (hash.finish() % self.num_shards as u64) as usize;
        &self.shards[index]
    }

    pub async fn get_write_lock<BK>(&self, key: &BK) -> RwLockWriteGuard<'_, HashMap<K, V>>
    where
        BK: Hash + Eq + ?Sized,
        K: Borrow<BK>
    {
        let shard = self.get_shard(key);
        shard.write().await
    }

    pub async fn get_read_lock<BK>(&self, key: &BK) -> RwLockReadGuard<'_, HashMap<K, V>>
    where
        BK: Hash + Eq + ?Sized,
        K: Borrow<BK>
    {
        let shard = self.get_shard(key);
        shard.read().await
    }

    pub async fn insert(&self, key: K, value: V) {
        let shard = self.get_shard(&key);
        let mut map = shard.write().await;
        map.insert(key, value);
    }
}