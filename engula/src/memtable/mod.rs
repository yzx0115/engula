mod btree_table;

pub use btree_table::BTreeTable;

use std::iter::Iterator;

use async_trait::async_trait;

use crate::format::Timestamp;

pub type MemItem<'a> = (Timestamp, &'a [u8], &'a [u8]);
pub type MemIter<'a> = dyn Iterator<Item = MemItem<'a>> + Send + Sync + 'a;

#[async_trait]
pub trait MemTable: Send + Sync {
    async fn get(&self, ts: Timestamp, key: &[u8]) -> Option<Vec<u8>>;

    async fn put(&self, ts: Timestamp, key: Vec<u8>, value: Vec<u8>);

    fn size(&self) -> usize;

    fn count(&self) -> usize;

    async fn snapshot(&self) -> Box<dyn MemSnapshot>;
}

pub trait MemSnapshot: Send + Sync {
    fn iter(&self) -> Box<MemIter>;
}
