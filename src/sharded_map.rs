use std::any::Any;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::Display;
use tokio::sync::{RwLock, RwLockWriteGuard, RwLockReadGuard};
use std::sync::Arc;
use std::hash::{Hash, Hasher};

type Inner = dyn Any + Send + Sync;
type Boxed = Box<Inner>;
type Shard<K> = HashMap<K, Boxed>;
#[derive(Default)]
pub struct ShardedMap<K> {
    shards: Vec<Arc<RwLock<Shard<K>>>>,
    num_shards: usize,
}

impl<K> ShardedMap<K> 
where
    K: Hash + Eq
{
    pub fn new(num_shards: usize) -> Self {
        let shards = (0..num_shards).map(|_| Arc::new(RwLock::new(HashMap::new()))).collect();

        ShardedMap { shards, num_shards}
    }

    fn get_shard<BK>(&self, key: &BK) -> &Arc<RwLock<Shard<K>>>
    where
        BK: Hash + Eq + ?Sized,
        K: Borrow<BK>
    {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        key.hash(&mut hash);
        let index = (hash.finish() % self.num_shards as u64) as usize;
        &self.shards[index]
    }

    /// Will Panic if types don't match
    pub async fn get<BK, T>(&self, key: &BK) -> Option<T>
    where
        BK: Hash + Eq + ?Sized + Display,
        K: Borrow<BK>,
        T: Clone + 'static
    {
        let shard = self.get_shard(key).read().await;
        shard.get(key).and_then(|value|{
            let value = value.downcast_ref::<T>();
            assert!(value.is_some(), "CrudeCache get: keys '{}' value is not type: {:?}", key, std::any::type_name::<T>());
            value
        }).cloned()
    }

    pub async fn get_write_lock<BK>(&self, key: &BK) -> RwLockWriteGuard<'_, Shard<K>>
    where
        BK: Hash + Eq + ?Sized,
        K: Borrow<BK>
    {
        let shard = self.get_shard(key);
        shard.write().await
    }

    pub async fn get_read_lock<BK>(&self, key: &BK) -> RwLockReadGuard<'_, Shard<K>>
    where
        BK: Hash + Eq + ?Sized,
        K: Borrow<BK>
    {
        let shard = self.get_shard(key);
        shard.read().await
    }

    pub async fn insert<T: Any + Send + Sync + 'static>(&self, key: K, value: T)
    {
        let shard = self.get_shard(&key);
        let lock = shard.write().await;
        Self::insert_with_lock(value, key, lock);
    }

    pub fn insert_with_lock<T: Any + Send + Sync + 'static>(value: T, key: K, mut lock: RwLockWriteGuard<'_, Shard<K>>) {
        lock.insert(key, Box::new(value) as Boxed);
    }

    pub async fn remove<BK>(&self, key: &BK) -> bool
    where
        BK: Hash + Eq + ?Sized,
        K: Borrow<BK>
    {
        let shard = self.get_shard(key);
        let mut locked_shard = shard.write().await;
        locked_shard.remove(key).is_some()
    }

}