use std::{sync::Arc, time::Instant};

use clap::Clap;
use engula::*;
use tokio::sync::Barrier;

#[derive(Clap, Debug)]
pub struct Command {
    // Database options
    #[clap(long, default_value = "8")]
    num_shards: usize,
    #[clap(long, default_value = "4")]
    num_levels: usize,
    #[clap(long, default_value = "16")]
    block_size_kb: usize,
    #[clap(long, default_value = "1024")]
    memtable_size_mb: usize,
    #[clap(long, default_value = "1024")]
    write_channel_size: usize,
    // Component options
    #[clap(long)]
    no_sync: bool,
    #[clap(long)]
    journal_url: String,
    #[clap(long)]
    storage_url: String,
    // Benchmark options
    #[clap(long)]
    do_get: bool,
    #[clap(long, default_value = "100000")]
    num_tasks: usize,
    #[clap(long, default_value = "1000000")]
    num_entries: usize,
    #[clap(long, default_value = "100")]
    value_size: usize,
}

impl Command {
    pub async fn run(&self) -> Result<()> {
        println!("{:#?}", self);
        let db = self.open().await?;
        let db = Arc::new(db);
        self.bench_put(db.clone()).await;
        if self.do_get {
            self.bench_get(db.clone()).await;
        }
        Ok(())
    }

    async fn open(&self) -> Result<Database> {
        let options = Options {
            num_shards: self.num_shards,
            memtable_size: self.memtable_size_mb * 1024 * 1024,
            write_channel_size: self.write_channel_size,
        };
        let manifest_options = ManifestOptions {
            num_shards: self.num_shards,
            num_levels: self.num_levels,
        };

        let journal = self.open_journal().await;
        let storage = self.open_storage().await;
        let runtime = Arc::new(LocalCompaction::new(storage.clone()));
        let manifest = Arc::new(LocalManifest::new(
            manifest_options,
            storage.clone(),
            runtime,
        ));

        Database::new(options, journal, storage, manifest).await
    }

    async fn open_journal(&self) -> Arc<dyn Journal> {
        let options = JournalOptions {
            sync: !self.no_sync,
            chunk_size: self.write_channel_size,
        };
        let journal = open_journal(&self.journal_url, options).await.unwrap();
        Arc::from(journal)
    }

    async fn open_storage(&self) -> Arc<dyn Storage> {
        let options = SstOptions {
            block_size: self.block_size_kb * 1024,
            block_cache: None,
        };
        let storage = SstStorage::new(&self.storage_url, options).await.unwrap();
        Arc::new(storage)
    }

    async fn bench_get(&self, db: Arc<Database>) {
        let mut tasks = Vec::new();
        let barrier = Arc::new(Barrier::new(self.num_tasks));

        let now = Instant::now();
        for _ in 0..self.num_tasks {
            let db_clone = db.clone();
            let barrier_clone = barrier.clone();
            let num_entries_per_task = self.num_entries / self.num_tasks;
            let task = tokio::task::spawn(async move {
                barrier_clone.wait().await;
                for i in 0..num_entries_per_task {
                    let key = i.to_be_bytes();
                    db_clone.get(&key).await.unwrap().unwrap();
                }
            });
            tasks.push(task);
        }
        for task in tasks {
            task.await.unwrap();
        }

        let elapsed = now.elapsed();
        println!("elapsed: {:?}", elapsed);
        println!("qps: {}", self.num_entries as f64 / elapsed.as_secs_f64());
    }

    async fn bench_put(&self, db: Arc<Database>) {
        let mut tasks = Vec::new();
        let barrier = Arc::new(Barrier::new(self.num_tasks));

        let now = Instant::now();
        for _ in 0..self.num_tasks {
            let mut value = Vec::new();
            value.resize(self.value_size, 0);
            let db_clone = db.clone();
            let barrier_clone = barrier.clone();
            let num_entries_per_task = self.num_entries / self.num_tasks;
            let task = tokio::task::spawn(async move {
                barrier_clone.wait().await;
                for i in 0..num_entries_per_task {
                    let key = i.to_be_bytes();
                    db_clone.put(key.to_vec(), value.clone()).await.unwrap();
                }
            });
            tasks.push(task);
        }
        for task in tasks {
            task.await.unwrap();
        }

        let elapsed = now.elapsed();
        println!("elapsed: {:?}", elapsed);
        println!("qps: {}", self.num_entries as f64 / elapsed.as_secs_f64());
    }
}
