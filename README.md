# CrudeCache

![Crates.io](https://img.shields.io/crates/v/crude_cache)
![Docs.rs](https://docs.rs/crude_cache/badge.svg)
![License](https://img.shields.io/crates/l/crude_cache)

`CrudeCache` is a simple utility for good enough concurrent caching needs. Inspired by Play Framework's cache.getOrElseUpdate.

```rust
//Returns non-expired cached item. If one does not exist, new item will be cached & returned
cache.get_or_else_update("cache_key", Duration::from_secs(60), async_function).await
```

**Disclaimer:** Please note that `CrudeCache` was developed for personal projects, it is not battle-tested in production and might contain bugs. Feedback, suggestions, and contributions are more than welcome.

## Features

- Expiration of cached items are evaluated lazily.
- To minimize contention points, items are stored in a `ShardedMap`, which splits the data across multiple shards (`HashMaps`). No bells and whistles, and no resharding.
- Each shard is behind `tokio::sync::RwLock`

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
crude_cache = "0.1.0"
```

## Example usage

```rust
use crude_cache::CrudeCache;

#[derive(Clone)]
pub struct DataService {
    cache: Arc<CrudeCache>,
    db: Arc<Db>
}

impl DataService {
    pub async fn get_big_data(&self) -> SomeBigType {
        self.cache.get_or_else_update("big_data", Duration::from_secs(60), || self.db.big_data()).await
    }

    pub async fn get_other_data(&self) -> OtherType {
        self.cache.get_or_else_update("other_data", Duration::from_secs(60), || self.db.other_data()).await
    }
}
```

## Similar Crates

More sophisticated alternative:

- [cached](https://crates.io/crates/cached)

