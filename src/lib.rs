pub mod error;
pub mod registry;
pub mod integrity;
pub mod models;
pub mod download_manager;
pub mod progress_bar;
pub mod smart_manager;
pub mod logger;
pub mod macros;

pub use models::{DownloadConfig, DownloadItem, DownloadStatus};
pub use smart_manager::{SmartManager, Verbosity};
pub use logger::{init_file_logging, FileLogger};



pub use macros::*;


