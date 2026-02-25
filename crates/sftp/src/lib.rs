mod russh_impl;

use anyhow::Result;
use async_trait::async_trait;
use ssh::SshConnectConfig;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::SystemTime;

pub use russh_impl::RusshSftpClient;

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified: SystemTime,
    pub is_dir: bool,
    pub permissions: u32,
}

#[derive(Debug, Clone)]
pub struct TransferItem {
    pub local_path: String,
    pub remote_path: String,
    pub size: u64,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TransferProgress {
    pub transferred: u64,
    pub total: u64,
    pub speed: f64,
    pub current_file: Option<String>,
    pub current_file_transferred: u64,
    pub current_file_total: u64,
}

#[derive(Debug)]
pub struct TransferCancelled;

impl std::fmt::Display for TransferCancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cancelled")
    }
}

impl std::error::Error for TransferCancelled {}

pub type ProgressCallback = Box<dyn Fn(TransferProgress) + Send + Sync + 'static>;

#[async_trait]
pub trait SftpClient: Send + Sync {
    async fn connect(ssh_config: SshConnectConfig) -> Result<Self>
    where
        Self: Sized;

    async fn list_dir(&mut self, path: &str) -> Result<Vec<FileEntry>>;

    async fn download_with_progress(
        &mut self,
        remote_path: &str,
        local_path: &str,
        cancelled: Arc<AtomicBool>,
        progress: ProgressCallback,
    ) -> Result<()>;

    async fn upload_with_progress(
        &mut self,
        local_path: &str,
        remote_path: &str,
        cancelled: Arc<AtomicBool>,
        progress: ProgressCallback,
    ) -> Result<()>;

    async fn delete(&mut self, path: &str, is_dir: bool) -> Result<()>;

    /// 递归删除目录及其所有内容，带进度回调
    async fn delete_recursive(
        &mut self,
        path: &str,
        cancelled: Arc<AtomicBool>,
        progress: ProgressCallback,
    ) -> Result<()>;

    async fn mkdir(&mut self, path: &str) -> Result<()>;

    async fn rename(&mut self, old_path: &str, new_path: &str) -> Result<()>;

    async fn chmod(&mut self, path: &str, mode: u32) -> Result<()>;

    /// 写入文件内容（用于创建新文件或覆盖文件）
    async fn write_file(&mut self, path: &str, content: &[u8]) -> Result<()>;

    async fn list_dir_recursive(
        &mut self,
        path: &str,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Vec<FileEntry>>;

    async fn download_dir_with_progress(
        &mut self,
        remote_path: &str,
        local_path: &str,
        cancelled: Arc<AtomicBool>,
        progress: ProgressCallback,
    ) -> Result<()>;

    async fn upload_dir_with_progress(
        &mut self,
        local_path: &str,
        remote_path: &str,
        cancelled: Arc<AtomicBool>,
        progress: ProgressCallback,
    ) -> Result<()>;

    async fn disconnect(&mut self) -> Result<()>;

    /// 获取路径的真实绝对路径
    async fn realpath(&mut self, path: &str) -> Result<String>;
}
