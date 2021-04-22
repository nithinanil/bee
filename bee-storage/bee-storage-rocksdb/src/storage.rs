// Copyright 2020-2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use super::{
    config::{RocksDbConfig, RocksDbConfigBuilder, StorageConfig},
    error::Error,
    system::{System, STORAGE_HEALTH_KEY, STORAGE_VERSION, STORAGE_VERSION_KEY},
};

pub use bee_storage::{
    access::{Fetch, Insert},
    backend::StorageBackend,
    health::StorageHealth,
};

use bee_message::{
    address::ED25519_ADDRESS_LENGTH, milestone::MilestoneIndex, payload::indexation::INDEXATION_PADDED_INDEX_LENGTH,
    MESSAGE_ID_LENGTH,
};

use async_trait::async_trait;
use rocksdb::{
    ColumnFamilyDescriptor, DBCompactionStyle, DBCompressionType, Env, FlushOptions, Options, SliceTransform, DB,
};

pub const CF_SYSTEM: &str = "system";
pub const CF_MESSAGE_ID_TO_MESSAGE: &str = "message_id_to_message";
pub const CF_MESSAGE_ID_TO_METADATA: &str = "message_id_to_metadata";
pub const CF_MESSAGE_ID_TO_MESSAGE_ID: &str = "message_id_to_message_id";
pub const CF_INDEX_TO_MESSAGE_ID: &str = "index_to_message_id";
pub const CF_OUTPUT_ID_TO_CREATED_OUTPUT: &str = "output_id_to_created_output";
pub const CF_OUTPUT_ID_TO_CONSUMED_OUTPUT: &str = "output_id_to_consumed_output";
pub const CF_OUTPUT_ID_UNSPENT: &str = "output_id_unspent";
pub const CF_ED25519_ADDRESS_TO_OUTPUT_ID: &str = "ed25519_address_to_output_id";
pub const CF_LEDGER_INDEX: &str = "ledger_index";
pub const CF_MILESTONE_INDEX_TO_MILESTONE: &str = "milestone_index_to_milestone";
pub const CF_SNAPSHOT_INFO: &str = "snapshot_info";
pub const CF_SOLID_ENTRY_POINT_TO_MILESTONE_INDEX: &str = "solid_entry_point_to_milestone_index";
pub const CF_MILESTONE_INDEX_TO_OUTPUT_DIFF: &str = "milestone_index_to_output_diff";
pub const CF_ADDRESS_TO_BALANCE: &str = "address_to_balance";
pub const CF_MILESTONE_INDEX_TO_UNCONFIRMED_MESSAGE: &str = "milestone_index_to_unconfirmed_message";
pub const CF_MILESTONE_INDEX_TO_RECEIPT: &str = "milestone_index_to_receipt";
pub const CF_SPENT_TO_TREASURY_OUTPUT: &str = "spent_to_treasury_output";

pub struct Storage {
    pub(crate) config: StorageConfig,
    pub(crate) inner: DB,
}

impl Storage {
    fn new(config: RocksDbConfig) -> Result<DB, Error> {
        let cf_system = ColumnFamilyDescriptor::new(CF_SYSTEM, Options::default());

        let cf_message_id_to_message = ColumnFamilyDescriptor::new(CF_MESSAGE_ID_TO_MESSAGE, Options::default());

        let cf_message_id_to_metadata = ColumnFamilyDescriptor::new(CF_MESSAGE_ID_TO_METADATA, Options::default());

        let prefix_extractor = SliceTransform::create_fixed_prefix(MESSAGE_ID_LENGTH);
        let mut options = Options::default();
        options.set_prefix_extractor(prefix_extractor);
        let cf_message_id_to_message_id = ColumnFamilyDescriptor::new(CF_MESSAGE_ID_TO_MESSAGE_ID, options);

        let prefix_extractor = SliceTransform::create_fixed_prefix(INDEXATION_PADDED_INDEX_LENGTH);
        let mut options = Options::default();
        options.set_prefix_extractor(prefix_extractor);
        let cf_index_to_message_id = ColumnFamilyDescriptor::new(CF_INDEX_TO_MESSAGE_ID, options);

        let cf_output_id_to_created_output =
            ColumnFamilyDescriptor::new(CF_OUTPUT_ID_TO_CREATED_OUTPUT, Options::default());

        let cf_output_id_to_consumed_output =
            ColumnFamilyDescriptor::new(CF_OUTPUT_ID_TO_CONSUMED_OUTPUT, Options::default());

        let cf_output_id_unspent = ColumnFamilyDescriptor::new(CF_OUTPUT_ID_UNSPENT, Options::default());

        let prefix_extractor = SliceTransform::create_fixed_prefix(ED25519_ADDRESS_LENGTH);
        let mut options = Options::default();
        options.set_prefix_extractor(prefix_extractor);
        let cf_ed25519_address_to_output_id = ColumnFamilyDescriptor::new(CF_ED25519_ADDRESS_TO_OUTPUT_ID, options);

        let cf_ledger_index = ColumnFamilyDescriptor::new(CF_LEDGER_INDEX, Options::default());

        let cf_milestone_index_to_milestone =
            ColumnFamilyDescriptor::new(CF_MILESTONE_INDEX_TO_MILESTONE, Options::default());

        let cf_snapshot_info = ColumnFamilyDescriptor::new(CF_SNAPSHOT_INFO, Options::default());

        let cf_solid_entry_point_to_milestone_index =
            ColumnFamilyDescriptor::new(CF_SOLID_ENTRY_POINT_TO_MILESTONE_INDEX, Options::default());

        let cf_milestone_index_to_output_diff =
            ColumnFamilyDescriptor::new(CF_MILESTONE_INDEX_TO_OUTPUT_DIFF, Options::default());

        let cf_address_to_balance = ColumnFamilyDescriptor::new(CF_ADDRESS_TO_BALANCE, Options::default());

        let prefix_extractor = SliceTransform::create_fixed_prefix(std::mem::size_of::<MilestoneIndex>());
        let mut options = Options::default();
        options.set_prefix_extractor(prefix_extractor);
        let cf_milestone_index_to_unconfirmed_message =
            ColumnFamilyDescriptor::new(CF_MILESTONE_INDEX_TO_UNCONFIRMED_MESSAGE, options);

        let prefix_extractor = SliceTransform::create_fixed_prefix(std::mem::size_of::<MilestoneIndex>());
        let mut options = Options::default();
        options.set_prefix_extractor(prefix_extractor);
        let cf_milestone_index_to_receipt = ColumnFamilyDescriptor::new(CF_MILESTONE_INDEX_TO_RECEIPT, options);

        let prefix_extractor = SliceTransform::create_fixed_prefix(std::mem::size_of::<bool>());
        let mut options = Options::default();
        options.set_prefix_extractor(prefix_extractor);
        let cf_spent_to_treasury = ColumnFamilyDescriptor::new(CF_SPENT_TO_TREASURY_OUTPUT, options);

        let mut opts = Options::default();

        opts.create_if_missing(config.create_if_missing);
        opts.create_missing_column_families(config.create_missing_column_families);
        if config.enable_statistics {
            opts.enable_statistics();
        }
        opts.increase_parallelism(config.increase_parallelism);
        opts.optimize_for_point_lookup(config.optimize_for_point_lookup);
        opts.optimize_level_style_compaction(config.optimize_level_style_compaction);
        opts.optimize_universal_style_compaction(config.optimize_universal_style_compaction);
        opts.set_advise_random_on_open(config.set_advise_random_on_open);
        opts.set_allow_concurrent_memtable_write(config.set_allow_concurrent_memtable_write);
        opts.set_allow_mmap_reads(config.set_allow_mmap_reads);
        opts.set_allow_mmap_writes(config.set_allow_mmap_writes);
        opts.set_atomic_flush(config.set_atomic_flush);
        opts.set_bytes_per_sync(config.set_bytes_per_sync);
        opts.set_compaction_readahead_size(config.set_compaction_readahead_size);
        opts.set_compaction_style(DBCompactionStyle::from(config.set_compaction_style));
        opts.set_max_write_buffer_number(config.set_max_write_buffer_number);
        opts.set_write_buffer_size(config.set_write_buffer_size);
        opts.set_db_write_buffer_size(config.set_db_write_buffer_size);
        opts.set_disable_auto_compactions(config.set_disable_auto_compactions);
        opts.set_compression_type(DBCompressionType::from(config.set_compression_type));
        opts.set_unordered_write(config.set_unordered_write);
        opts.set_use_direct_io_for_flush_and_compaction(config.set_use_direct_io_for_flush_and_compaction);

        let mut env = Env::default()?;
        env.set_background_threads(config.env.set_background_threads);
        env.set_high_priority_background_threads(config.env.set_high_priority_background_threads);
        opts.set_env(&env);

        let column_familes = vec![
            cf_system,
            cf_message_id_to_message,
            cf_message_id_to_metadata,
            cf_message_id_to_message_id,
            cf_index_to_message_id,
            cf_output_id_to_created_output,
            cf_output_id_to_consumed_output,
            cf_output_id_unspent,
            cf_ed25519_address_to_output_id,
            cf_ledger_index,
            cf_milestone_index_to_milestone,
            cf_snapshot_info,
            cf_solid_entry_point_to_milestone_index,
            cf_milestone_index_to_output_diff,
            cf_address_to_balance,
            cf_milestone_index_to_unconfirmed_message,
            cf_milestone_index_to_receipt,
            cf_spent_to_treasury,
        ];

        let db = DB::open_cf_descriptors(&opts, config.path, column_familes)?;

        let mut flushopts = FlushOptions::new();
        flushopts.set_wait(true);
        db.flush_opt(&flushopts)?;
        db.flush_cf_opt(db.cf_handle(CF_SYSTEM).unwrap(), &flushopts)?;

        Ok(db)
    }
}

#[async_trait]
impl StorageBackend for Storage {
    type ConfigBuilder = RocksDbConfigBuilder;
    type Config = RocksDbConfig;
    type Error = Error;

    /// It starts RocksDb instance and then initializes the required column familes.
    async fn start(config: Self::Config) -> Result<Self, Self::Error> {
        let storage = Storage {
            config: config.storage.clone(),
            inner: Self::new(config)?,
        };

        match Fetch::<u8, System>::fetch(&storage, &STORAGE_VERSION_KEY).await? {
            Some(System::Version(version)) => {
                if version != STORAGE_VERSION {
                    return Err(Error::VersionMismatch(version, STORAGE_VERSION));
                }
            }
            None => {
                Insert::<u8, System>::insert(&storage, &STORAGE_VERSION_KEY, &System::Version(STORAGE_VERSION)).await?
            }
            _ => panic!("Another system value was inserted on the version key."),
        }

        if let Some(health) = storage.get_health().await? {
            if health != StorageHealth::Healthy {
                return Err(Self::Error::UnhealthyStorage(health));
            }
        }

        storage.set_health(StorageHealth::Idle).await?;

        Ok(storage)
    }

    /// It shutdowns RocksDb instance.
    /// Note: the shutdown is done through flush method and then droping the storage object.
    async fn shutdown(self) -> Result<(), Self::Error> {
        self.set_health(StorageHealth::Healthy).await?;

        Ok(self.inner.flush()?)
    }

    async fn size(&self) -> Result<Option<usize>, Self::Error> {
        let mut size = 0;
        let files = self.inner.live_files()?;

        for file in files {
            size += file.size
        }

        Ok(Some(size))
    }

    async fn get_health(&self) -> Result<Option<StorageHealth>, Self::Error> {
        Ok(match Fetch::<u8, System>::fetch(self, &STORAGE_HEALTH_KEY).await? {
            Some(System::Health(health)) => Some(health),
            None => None,
            _ => panic!("Another system value was inserted on the health key."),
        })
    }

    async fn set_health(&self, health: StorageHealth) -> Result<(), Self::Error> {
        Insert::<u8, System>::insert(self, &STORAGE_HEALTH_KEY, &System::Health(health)).await
    }
}
