#![deny(clippy::all)]

//! # `CrudeCache`
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
mod crude_cache;

pub use crate::crude_cache::CrudeCache;