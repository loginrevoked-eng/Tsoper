use crate::download_manager::{download_file, load_tracker, save_tracker};
use crate::progress_bar::ProgressBar;
use crate::integrity::verify_file_integrity;
use std::fs;
use crate::{dprint, vprintln, dprintln, derrprintln};
use crate::models::{DownloadConfig, DownloadItem, DownloadStatus};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verbosity {
    NoConsole,
    Verbose,
    Debug,
    Normal,
}

pub struct SmartManager {
    name: String,
    tracking_file: String,
    tracking_info: DownloadConfig,
    is_first_run: bool,
}

// ==========================================
// Initialization and State Tracking Clusters
// ==========================================
impl SmartManager {
    pub fn new(name: String, tracking_file: Option<String>, registry_key_path: Option<String>) -> Self {
        let registry_path = registry_key_path.unwrap_or_else(|| "SOFTWARE\\Dowman".to_string());

        // 1. Resolve Path: CLI input always takes precedence and triggers a registry update
        let (tracking_file_path, force_registry_update) = if let Some(input_path) = tracking_file {
            let path_buf = std::path::Path::new(&input_path);
            let resolved = if path_buf.is_absolute() {
                input_path
            } else {
                std::env::current_dir()
                    .map(|cwd| cwd.join(&input_path).to_string_lossy().into_owned())
                    .unwrap_or(input_path)
            };
            (resolved, true) 
        } else {
            // No CLI: Check registry (Source of Truth)
            match crate::registry::get_from_registry(&registry_path, "TrackingFilePath") {
                Ok(reg_path) => (reg_path, false),
                Err(_) => {
                    // Fallback: CWD + default filename
                    let default_path = std::env::current_dir()
                        .map(|cwd| cwd.join("my_downloads.json").to_string_lossy().into_owned())
                        .unwrap_or_else(|_| "my_downloads.json".to_string());
                    (default_path, true)
                }
            }
        };

        // 2. Check existence on disk
        let is_first_run = fs::metadata(&tracking_file_path).is_err();

        let tracking_info = if !is_first_run {
            load_tracker(&tracking_file_path).unwrap_or_else(|_| DownloadConfig {
                name: "Dowman Download Tracker".to_string(),
                downloads: vec![],
            })
        } else {
            DownloadConfig {
                name: "Dowman Download Tracker".to_string(),
                downloads: vec![],
            }
        };

        let manager = Self {
            name,
            tracking_file: tracking_file_path.clone(),
            tracking_info,
            is_first_run,
        };

        // 3. Update Registry if forced by CLI or if the file is new
        if force_registry_update || is_first_run {
            let _ = manager.save_state();
            if let Err(e) = crate::registry::add_to_registry(&registry_path, "TrackingFilePath", &tracking_file_path) {
                derrprintln!("Failed to set registry key: {}", e);
            }
        }

        manager
    }
}


impl SmartManager {
    pub fn get_status(&self) -> &DownloadConfig {
        &self.tracking_info
    }

    pub fn save_state(&self) -> crate::error::Result<()> {
        save_tracker(&self.tracking_info, &self.tracking_file)
    }

    pub fn is_first_run(&self) -> bool {
        self.is_first_run
    }
    pub fn add_download(&mut self, id: String, name: String, url: String, destination: String) {
        if let Some(existing_download) = self.tracking_info.downloads.iter_mut().find(|d| d.id == id) {
            let url_changed = existing_download.url != url;
            let dest_changed = existing_download.destination != destination;
            
            existing_download.name = name;
            existing_download.url = url;
            existing_download.destination = destination;
            
            if matches!(existing_download.status_info, DownloadStatus::Failed { .. }) || url_changed || dest_changed {
                existing_download.status_info = DownloadStatus::NotStarted;
            }
        } else {
            let new_download = DownloadItem {
                id: id.clone(),
                name,
                url,
                destination,
                status_info: DownloadStatus::NotStarted,
            };
            
            self.tracking_info.downloads.push(new_download);
        }
        
        if let Err(e) = self.save_state() {
            let _verbosity = Verbosity::Debug;
            derrprintln!("Failed to save state after adding download {}: {}", id, e);
        }
    }
}


// ==========================================
// Core Download Execution Clusters
// ==========================================
impl SmartManager {
    pub fn start_downloads(&mut self) -> crate::error::Result<()> {
        self.start_downloads_with_verbosity(crate::Verbosity::Normal)
    }

    pub fn start_downloads_with_verbosity(&mut self, verbosity: crate::Verbosity) -> crate::error::Result<()> {
        vprintln!("VERBOSE MODE: Starting download manager: {}", self.name);
        dprintln!("DEBUG MODE: Starting download manager: {}", self.name);
        vprintln!("Starting download manager: {}", self.name);
        dprintln!("Starting download manager: {}", self.name);
        
        let all_completed = self.are_all_downloads_completed();
        
        if all_completed {
            if self.verify_all_completed_downloads() {
                vprintln!("All downloads are already completed and verified. Exiting.");
                dprintln!("All downloads are already completed and verified. Exiting.");
                return Ok(());
            } else {
                vprintln!("Some downloads need re-downloading due to integrity issues.");
                dprintln!("Some downloads need re-downloading due to integrity issues.");
            }
        }

        self.process_pending_downloads(verbosity)?;

        if self.are_all_downloads_completed() {
            vprintln!("\nAll downloads completed successfully!");
            dprintln!("\nAll downloads completed successfully!");
            pr_finished_callnext();
        } else {
            vprintln!("\nSome downloads failed or are incomplete.");
            dprintln!("\nSome downloads failed or are incomplete.");
        }

        Ok(())
    }

    fn download_with_progress(&mut self, download_index: usize, verbosity: Verbosity) -> crate::error::Result<()> {
        let download_name = self.tracking_info.downloads[download_index].name.clone();
        let download_url = self.tracking_info.downloads[download_index].url.clone();
        let download_destination = self.tracking_info.downloads[download_index].destination.clone();
        
        if download_url.is_empty() {
            dprint!("Skipping {} - no URL provided", download_name);
            return Ok(());
        }

        self.tracking_info.downloads[download_index].status_info = DownloadStatus::InProgress;
        if let Err(e) = self.save_state() {
            derrprintln!("Failed to save state before download: {}", e);
        }

        let progress = if !matches!(verbosity, Verbosity::NoConsole) {
            Some(ProgressBar::new(100, download_name.clone(), Some(4)))
        } else {
            None
        };

        if let Some(ref p) = progress {
            p.start();
        }
        
        match download_file(&download_url, &download_destination) {
            Ok((_actual_size, expected_size)) => {
                if let Ok(metadata) = std::fs::metadata(&download_destination) {
                    let current_size = metadata.len();
                    if let Some(exp) = expected_size {
                        if current_size >= exp {
                            self.tracking_info.downloads[download_index].status_info = DownloadStatus::Completed;
                        } else {
                            self.tracking_info.downloads[download_index].status_info = DownloadStatus::Partial { 
                                bytes_downloaded: current_size, 
                                total_bytes: exp 
                            };
                        }
                    } else {
                        self.tracking_info.downloads[download_index].status_info = DownloadStatus::Completed;
                    }
                } else {
                    self.tracking_info.downloads[download_index].status_info = DownloadStatus::Completed;
                }
                
                if let Some(p) = progress {
                    p.finish(false);
                }
                dprint!("Completed: {}", download_name);
            }
            Err(e) => {
                let err_msg = e.to_string();
                self.tracking_info.downloads[download_index].status_info = DownloadStatus::Failed { 
                    error_message: Some(err_msg.clone()) 
                };
                if let Some(p) = progress {
                    p.error(Some(&err_msg));
                }
                dprint!("Failed: {} - {}", download_name, err_msg);
                dw_failed_dummycall();
            }
        }

        if let Err(e) = self.save_state() {
            derrprintln!("Failed to save state after download {}: {}", download_name, e);
        }
        Ok(())
    }
}


// ==========================================
// Validation and Processing Helpers Clusters
// ==========================================
impl SmartManager {
    fn are_all_downloads_completed(&self) -> bool {
        self.tracking_info.downloads.iter()
            .all(|d| matches!(d.status_info, DownloadStatus::Completed))
    }

    fn verify_all_completed_downloads(&self) -> bool {
        self.tracking_info.downloads.iter()
            .all(|d| verify_file_integrity(d))
    }

    fn process_pending_downloads(&mut self, verbosity: crate::Verbosity) -> crate::error::Result<()> {
        let download_indices: Vec<usize> = (0..self.tracking_info.downloads.len()).collect();
        
        for index in download_indices {
            let status = self.tracking_info.downloads[index].status_info.clone();
            
            if self.requires_download(index, &status) {
                self.download_with_progress(index, verbosity)?;
            }
        }
        Ok(())
    }

    fn requires_download(&mut self, index: usize, status: &DownloadStatus) -> bool {
        match status {
            DownloadStatus::NotStarted => true,
            DownloadStatus::InProgress => false, // Already downloading
            DownloadStatus::Completed | DownloadStatus::Partial { .. } => {
                if !verify_file_integrity(&self.tracking_info.downloads[index]) {
                    dprintln!("Integrity check failed for {}, resetting status", self.tracking_info.downloads[index].name);
                    self.reset_download_status(index);
                    true
                } else if matches!(status, DownloadStatus::Partial { .. }) {
                    true // Valid partial file, but still needs to finish
                } else {
                    false // Completed and valid
                }
            },
            DownloadStatus::Failed { .. } => {
                self.reset_download_status(index);
                true // Retry failed downloads
            }
        }
    }

    fn reset_download_status(&mut self, index: usize) {
        self.tracking_info.downloads[index].status_info = DownloadStatus::NotStarted;
        if let Err(e) = self.save_state() {
            eprintln!("Failed to save state before re-download: {}", e);
        }
    }
}

pub fn dw_failed_dummycall() {
    dprintln!("DUMMY JOBLESS FUNCTION CALLED");
}

pub fn pr_finished_callnext() {
    dprintln!("🎉 ALL DOWNLOADS COMPLETE - TIME TO PARTY LIKE IT'S 1999! 🎉");
}
