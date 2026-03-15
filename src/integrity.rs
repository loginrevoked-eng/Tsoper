use crate::models::{DownloadItem, DownloadStatus};
use crate::dprintln;

pub fn verify_file_integrity(download: &DownloadItem) -> bool {
    // Check if file exists and matches tracking info
    match std::path::Path::new(&download.destination).exists() {
        true => {
            // Get file size on disk
            if let Ok(metadata) = std::fs::metadata(&download.destination) {
                let file_size = metadata.len();
                
                // For completed downloads, verify file exists and is not totally empty
                // Note: since Completed doesn't store size, we just do a basic sanity check
                match &download.status_info {
                    DownloadStatus::Completed => {
                        if file_size > 0 {
                            dprintln!("✓ File integrity verified: {} ({} bytes)", download.name, file_size);
                            true
                        } else {
                            dprintln!("⚠ Completed file is 0 bytes: {}", download.name);
                            false
                        }
                    },
                    DownloadStatus::Partial { bytes_downloaded: tracked_bytes, total_bytes: _ } => {
                        if file_size == *tracked_bytes {
                            dprintln!("✓ Partial file integrity verified: {} ({} bytes)", download.name, file_size);
                            true
                        } else {
                            dprintln!("⚠ Partial file size mismatch for {}: disk={} vs tracked={}", 
                                     download.name, file_size, tracked_bytes);
                            false
                        }
                    },
                    _ => {
                        dprintln!("⚠ File exists but tracking status unexpected for: {}", download.name);
                        false
                    }
                }
            } else {
                dprintln!("⚠ Cannot read metadata for: {}", download.name);
                false
            }
        },
        false => {
            dprintln!("⚠ File not found on disk: {}", download.name);
            false
        }
    }
}
