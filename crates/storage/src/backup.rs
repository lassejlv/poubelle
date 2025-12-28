//! S3 backup functionality for Poubelle storage engine.
//!
//! This module provides backup and restore capabilities using AWS S3.
//! Enable with the `s3-backup` feature flag.
//!
//! # Example
//!
//! ```ignore
//! use storage::backup::{S3BackupConfig, S3Backup};
//!
//! let config = S3BackupConfig::new("my-bucket", "backups/poubelle/");
//! let backup = S3Backup::new(config).await?;
//!
//! // Create a backup
//! let backup_id = backup.backup_storage(&storage).await?;
//!
//! // Restore from a backup
//! backup.restore_storage(&backup_id, &restore_path).await?;
//! ```

use crate::StorageError;
use aws_config::BehaviorVersion;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BackupError {
    #[error("S3 error: {0}")]
    S3(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Backup not found: {0}")]
    BackupNotFound(String),
    #[error("Invalid backup manifest")]
    InvalidManifest,
}

/// Configuration for S3 backups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3BackupConfig {
    /// S3 bucket name
    pub bucket: String,
    /// Prefix/path within the bucket for backups
    pub prefix: String,
    /// Optional custom endpoint (for S3-compatible services like MinIO)
    pub endpoint: Option<String>,
    /// AWS region (defaults to us-east-1)
    pub region: String,
}

impl S3BackupConfig {
    /// Create a new S3 backup configuration
    pub fn new(bucket: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            prefix: prefix.into(),
            endpoint: None,
            region: "us-east-1".to_string(),
        }
    }

    /// Set a custom endpoint (for MinIO, LocalStack, etc.)
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    /// Set the AWS region
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = region.into();
        self
    }
}

/// Metadata about a backup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Unique backup identifier
    pub id: String,
    /// Timestamp when backup was created
    pub created_at: chrono::DateTime<Utc>,
    /// List of files included in the backup
    pub files: Vec<String>,
    /// Total size in bytes
    pub total_size: u64,
}

/// S3 backup handler
pub struct S3Backup {
    client: Client,
    config: S3BackupConfig,
}

impl S3Backup {
    /// Create a new S3 backup handler
    pub async fn new(config: S3BackupConfig) -> Result<Self, BackupError> {
        let mut sdk_config_loader =
            aws_config::defaults(BehaviorVersion::latest()).region(aws_config::Region::new(
                config.region.clone(),
            ));

        if let Some(endpoint) = &config.endpoint {
            sdk_config_loader = sdk_config_loader.endpoint_url(endpoint);
        }

        let sdk_config = sdk_config_loader.load().await;
        let client = Client::new(&sdk_config);

        Ok(Self { client, config })
    }

    /// Create a backup of the storage directory to S3
    pub async fn backup_storage(&self, storage_path: &Path) -> Result<BackupManifest, BackupError> {
        let backup_id = format!("backup-{}", Utc::now().format("%Y%m%d-%H%M%S"));
        let mut files = Vec::new();
        let mut total_size = 0u64;

        // Collect all files to backup
        let entries = self.collect_files(storage_path)?;

        for (relative_path, full_path) in &entries {
            let content = std::fs::read(full_path)?;
            total_size += content.len() as u64;

            let s3_key = format!("{}{}/{}", self.config.prefix, backup_id, relative_path);

            self.client
                .put_object()
                .bucket(&self.config.bucket)
                .key(&s3_key)
                .body(ByteStream::from(content))
                .send()
                .await
                .map_err(|e| BackupError::S3(e.to_string()))?;

            files.push(relative_path.clone());
        }

        let manifest = BackupManifest {
            id: backup_id.clone(),
            created_at: Utc::now(),
            files,
            total_size,
        };

        // Upload manifest
        let manifest_bytes = bincode::serialize(&manifest)?;
        let manifest_key = format!("{}{}/manifest.bin", self.config.prefix, backup_id);

        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(&manifest_key)
            .body(ByteStream::from(manifest_bytes))
            .send()
            .await
            .map_err(|e| BackupError::S3(e.to_string()))?;

        Ok(manifest)
    }

    /// Restore a backup from S3 to a local directory
    pub async fn restore_storage(
        &self,
        backup_id: &str,
        restore_path: &Path,
    ) -> Result<BackupManifest, BackupError> {
        // Download and parse manifest
        let manifest_key = format!("{}{}/manifest.bin", self.config.prefix, backup_id);

        let manifest_response = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&manifest_key)
            .send()
            .await
            .map_err(|e| BackupError::BackupNotFound(format!("{}: {}", backup_id, e)))?;

        let manifest_bytes = manifest_response
            .body
            .collect()
            .await
            .map_err(|e| BackupError::S3(e.to_string()))?
            .into_bytes();

        let manifest: BackupManifest =
            bincode::deserialize(&manifest_bytes).map_err(|_| BackupError::InvalidManifest)?;

        // Create restore directory
        std::fs::create_dir_all(restore_path)?;

        // Download each file
        for file in &manifest.files {
            let s3_key = format!("{}{}/{}", self.config.prefix, backup_id, file);

            let response = self
                .client
                .get_object()
                .bucket(&self.config.bucket)
                .key(&s3_key)
                .send()
                .await
                .map_err(|e| BackupError::S3(e.to_string()))?;

            let content = response
                .body
                .collect()
                .await
                .map_err(|e| BackupError::S3(e.to_string()))?
                .into_bytes();

            let file_path = restore_path.join(file);

            // Create parent directories if needed
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(&file_path, content)?;
        }

        Ok(manifest)
    }

    /// List all available backups
    pub async fn list_backups(&self) -> Result<Vec<BackupManifest>, BackupError> {
        let mut backups = Vec::new();

        let list_response = self
            .client
            .list_objects_v2()
            .bucket(&self.config.bucket)
            .prefix(&self.config.prefix)
            .delimiter("/")
            .send()
            .await
            .map_err(|e| BackupError::S3(e.to_string()))?;

        if let Some(common_prefixes) = list_response.common_prefixes {
            for prefix in common_prefixes {
                if let Some(prefix_str) = prefix.prefix {
                    // Extract backup ID from prefix
                    let backup_id = prefix_str
                        .trim_start_matches(&self.config.prefix)
                        .trim_end_matches('/');

                    if backup_id.starts_with("backup-") {
                        // Try to fetch manifest
                        let manifest_key = format!("{}manifest.bin", prefix_str);

                        if let Ok(response) = self
                            .client
                            .get_object()
                            .bucket(&self.config.bucket)
                            .key(&manifest_key)
                            .send()
                            .await
                        {
                            if let Ok(bytes) = response.body.collect().await {
                                if let Ok(manifest) =
                                    bincode::deserialize::<BackupManifest>(&bytes.into_bytes())
                                {
                                    backups.push(manifest);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Sort by creation date, newest first
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }

    /// Delete a backup from S3
    pub async fn delete_backup(&self, backup_id: &str) -> Result<(), BackupError> {
        // First get the manifest to know which files to delete
        let manifest_key = format!("{}{}/manifest.bin", self.config.prefix, backup_id);

        let manifest_response = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&manifest_key)
            .send()
            .await
            .map_err(|e| BackupError::BackupNotFound(format!("{}: {}", backup_id, e)))?;

        let manifest_bytes = manifest_response
            .body
            .collect()
            .await
            .map_err(|e| BackupError::S3(e.to_string()))?
            .into_bytes();

        let manifest: BackupManifest =
            bincode::deserialize(&manifest_bytes).map_err(|_| BackupError::InvalidManifest)?;

        // Delete all files
        for file in &manifest.files {
            let s3_key = format!("{}{}/{}", self.config.prefix, backup_id, file);

            self.client
                .delete_object()
                .bucket(&self.config.bucket)
                .key(&s3_key)
                .send()
                .await
                .map_err(|e| BackupError::S3(e.to_string()))?;
        }

        // Delete manifest
        self.client
            .delete_object()
            .bucket(&self.config.bucket)
            .key(&manifest_key)
            .send()
            .await
            .map_err(|e| BackupError::S3(e.to_string()))?;

        Ok(())
    }

    /// Collect all files in a directory recursively
    fn collect_files(&self, base_path: &Path) -> Result<Vec<(String, PathBuf)>, BackupError> {
        let mut files = Vec::new();
        self.collect_files_recursive(base_path, base_path, &mut files)?;
        Ok(files)
    }

    fn collect_files_recursive(
        &self,
        base_path: &Path,
        current_path: &Path,
        files: &mut Vec<(String, PathBuf)>,
    ) -> Result<(), BackupError> {
        if current_path.is_dir() {
            for entry in std::fs::read_dir(current_path)? {
                let entry = entry?;
                let path = entry.path();
                self.collect_files_recursive(base_path, &path, files)?;
            }
        } else if current_path.is_file() {
            let relative = current_path
                .strip_prefix(base_path)
                .unwrap_or(current_path)
                .to_string_lossy()
                .to_string();
            files.push((relative, current_path.to_path_buf()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = S3BackupConfig::new("test-bucket", "backups/")
            .with_region("eu-west-1")
            .with_endpoint("http://localhost:9000");

        assert_eq!(config.bucket, "test-bucket");
        assert_eq!(config.prefix, "backups/");
        assert_eq!(config.region, "eu-west-1");
        assert_eq!(config.endpoint, Some("http://localhost:9000".to_string()));
    }
}

