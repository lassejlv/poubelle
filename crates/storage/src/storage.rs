use crate::page::{Page, PAGE_SIZE};
use crate::types::{ColumnType, Row};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("Table not found: {0}")]
    TableNotFound(String),
    #[error("Table already exists: {0}")]
    TableExists(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TableMeta {
    pub name: String,
    pub columns: HashMap<String, ColumnType>,
    pub page_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct Catalog {
    tables: HashMap<String, TableMeta>,
}

pub struct Storage {
    path: PathBuf,
    catalog: Catalog,
    page_cache: DashMap<(String, usize), Page>,
}

impl Storage {
    pub fn open(path: PathBuf) -> Result<Self, StorageError> {
        let catalog_path = path.join("catalog.bin");

        let catalog = if catalog_path.exists() {
            let file = File::open(&catalog_path)?;
            let reader = BufReader::new(file);
            bincode::deserialize_from(reader)?
        } else {
            std::fs::create_dir_all(&path)?;
            Catalog {
                tables: HashMap::new(),
            }
        };

        Ok(Self {
            path,
            catalog,
            page_cache: DashMap::new(),
        })
    }

    fn save_catalog(&self) -> Result<(), StorageError> {
        let catalog_path = self.path.join("catalog.bin");
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(catalog_path)?;
        let writer = BufWriter::new(file);
        bincode::serialize_into(writer, &self.catalog)?;
        Ok(())
    }

    pub fn create_table(
        &mut self,
        name: String,
        columns: HashMap<String, ColumnType>,
    ) -> Result<(), StorageError> {
        if self.catalog.tables.contains_key(&name) {
            return Err(StorageError::TableExists(name));
        }

        let meta = TableMeta {
            name: name.clone(),
            columns,
            page_count: 0,
        };

        self.catalog.tables.insert(name, meta);
        self.save_catalog()?;
        Ok(())
    }

    pub fn drop_table(&mut self, name: &str) -> Result<(), StorageError> {
        if !self.catalog.tables.contains_key(name) {
            return Err(StorageError::TableNotFound(name.to_string()));
        }

        self.catalog.tables.remove(name);
        self.save_catalog()?;

        let table_path = self.path.join(format!("{}.bin", name));
        if table_path.exists() {
            std::fs::remove_file(table_path)?;
        }

        self.page_cache.retain(|key, _| key.0 != name);

        Ok(())
    }

    pub fn insert_row(&mut self, table: &str, row: Row) -> Result<(), StorageError> {
        let page_id = {
            let meta = self
                .catalog
                .tables
                .get_mut(table)
                .ok_or_else(|| StorageError::TableNotFound(table.to_string()))?;

            if meta.page_count == 0 {
                meta.page_count = 1;
                0
            } else {
                meta.page_count - 1
            }
        };

        let table_path = self.path.join(format!("{}.bin", table));
        let mut page = self
            .load_page(&table_path, page_id, table)
            .unwrap_or_else(|_| Page::new());

        page.add_row(row);

        let page_bytes = page.to_bytes()?;
        let needs_new_page = page_bytes.len() > PAGE_SIZE;

        self.save_page(&table_path, page_id, &page, table)?;

        if needs_new_page {
            let meta = self.catalog.tables.get_mut(table).unwrap();
            meta.page_count += 1;
        }

        self.save_catalog()?;
        Ok(())
    }

    pub fn scan_table(&self, table: &str) -> Result<Vec<Row>, StorageError> {
        let meta = self
            .catalog
            .tables
            .get(table)
            .ok_or_else(|| StorageError::TableNotFound(table.to_string()))?;

        let mut rows = Vec::with_capacity(meta.page_count * 10);
        let table_path = self.path.join(format!("{}.bin", table));

        for page_id in 0..meta.page_count {
            if let Ok(page) = self.load_page(&table_path, page_id, table) {
                rows.extend(page.rows);
            }
        }

        Ok(rows)
    }

    pub fn list_tables(&self) -> Vec<String> {
        self.catalog.tables.keys().cloned().collect()
    }

    pub fn get_table_meta(&self, table: &str) -> Option<&TableMeta> {
        self.catalog.tables.get(table)
    }

    fn load_page(
        &self,
        table_path: &PathBuf,
        page_id: usize,
        table: &str,
    ) -> Result<Page, StorageError> {
        let cache_key = (table.to_string(), page_id);

        if let Some(cached) = self.page_cache.get(&cache_key) {
            return Ok(cached.clone());
        }

        let file = File::open(table_path)?;
        let mut reader = BufReader::new(file);
        let offset = page_id * PAGE_SIZE;
        reader.seek(SeekFrom::Start(offset as u64))?;

        let mut buffer = vec![0u8; PAGE_SIZE];
        let bytes_read = reader.read(&mut buffer)?;
        buffer.truncate(bytes_read);

        if buffer.is_empty() {
            return Ok(Page::new());
        }

        let page = Page::from_bytes(&buffer)?;
        self.page_cache.insert(cache_key, page.clone());

        Ok(page)
    }

    fn save_page(
        &self,
        table_path: &PathBuf,
        page_id: usize,
        page: &Page,
        table: &str,
    ) -> Result<(), StorageError> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .read(true)
            .open(table_path)?;
        let mut writer = BufWriter::new(file);

        let offset = page_id * PAGE_SIZE;
        writer.seek(SeekFrom::Start(offset as u64))?;

        let mut bytes = page.to_bytes()?;
        bytes.resize(PAGE_SIZE, 0);
        writer.write_all(&bytes)?;
        writer.flush()?;

        let cache_key = (table.to_string(), page_id);
        self.page_cache.insert(cache_key, page.clone());

        Ok(())
    }

    /// Get the storage directory path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Flush all cached data to disk
    pub fn flush(&self) -> Result<(), StorageError> {
        self.save_catalog()
    }
}

// S3 backup integration methods
#[cfg(feature = "s3-backup")]
impl Storage {
    /// Create a backup of this storage to S3
    ///
    /// # Example
    ///
    /// ```ignore
    /// use storage::{Storage, S3BackupConfig};
    ///
    /// let storage = Storage::open("./data".into())?;
    /// let config = S3BackupConfig::new("my-bucket", "backups/");
    /// let manifest = storage.backup_to_s3(config).await?;
    /// println!("Backup created: {}", manifest.id);
    /// ```
    pub async fn backup_to_s3(
        &self,
        config: crate::backup::S3BackupConfig,
    ) -> Result<crate::backup::BackupManifest, crate::backup::BackupError> {
        // Ensure all data is flushed to disk before backup
        self.save_catalog()?;

        let backup = crate::backup::S3Backup::new(config).await?;
        backup.backup_storage(&self.path).await
    }

    /// Restore from an S3 backup to a new storage location
    ///
    /// # Example
    ///
    /// ```ignore
    /// use storage::{Storage, S3BackupConfig};
    ///
    /// let config = S3BackupConfig::new("my-bucket", "backups/");
    /// let storage = Storage::restore_from_s3(
    ///     config,
    ///     "backup-20240101-120000",
    ///     "./restored-data".into()
    /// ).await?;
    /// ```
    pub async fn restore_from_s3(
        config: crate::backup::S3BackupConfig,
        backup_id: &str,
        restore_path: PathBuf,
    ) -> Result<Self, crate::backup::BackupError> {
        let backup = crate::backup::S3Backup::new(config).await?;
        backup.restore_storage(backup_id, &restore_path).await?;
        Ok(Self::open(restore_path)?)
    }

    /// List all available S3 backups
    pub async fn list_s3_backups(
        config: crate::backup::S3BackupConfig,
    ) -> Result<Vec<crate::backup::BackupManifest>, crate::backup::BackupError> {
        let backup = crate::backup::S3Backup::new(config).await?;
        backup.list_backups().await
    }

    /// Delete an S3 backup
    pub async fn delete_s3_backup(
        config: crate::backup::S3BackupConfig,
        backup_id: &str,
    ) -> Result<(), crate::backup::BackupError> {
        let backup = crate::backup::S3Backup::new(config).await?;
        backup.delete_backup(backup_id).await
    }
}
