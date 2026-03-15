use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;
use std::os::windows::fs::OpenOptionsExt;

pub struct FileLogger {
    log_file: String,
}

impl FileLogger {
    pub fn new(log_file: &str) -> Self {
        Self {
            log_file: log_file.to_string(),
        }
    }

    pub fn log(&self, level: &str, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let log_entry = format!("{} [{}] {}\n", timestamp, level, message);
        
        if let Some(parent) = std::path::Path::new(&self.log_file).parent() {
            std::fs::create_dir_all(parent).unwrap_or(());
        }
        
        let mut options = OpenOptions::new();
        options.create(true).append(true);
        options.share_mode(0); // No sharing - exclusive access while using
        
        if let Ok(mut file) = options.open(&self.log_file) {
            let _ = file.write_all(log_entry.as_bytes());
        }
    }

    pub fn open_tracking_json_exclusive(&self) -> Result<std::fs::File, String> {
        if let Some(parent) = std::path::Path::new(&self.log_file).parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        
        let mut options = OpenOptions::new();
        options.create(true).write(true);
        options.share_mode(0); // No sharing - exclusive access
        
        options.open(&self.log_file).map_err(|e| e.to_string())
    }
}

pub fn init_file_logging(log_file: &str, _level: &str) -> Result<(), String> {
    // Just create the logger, level is ignored for simplicity
    let _logger = FileLogger::new(log_file);
    Ok(())
}
