use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub enum DownloadStatus {
    #[default]
    NotStarted,
    InProgress,
    Completed,
    Failed { error_message: Option<String> },
    Partial { bytes_downloaded: u64, total_bytes: u64 },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadItem {
    pub id: String,
    pub name: String,
    pub url: String,
    pub destination: String,
    pub status_info: DownloadStatus,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DownloadConfig {
    pub name: String,
    pub downloads: Vec<DownloadItem>,
}

