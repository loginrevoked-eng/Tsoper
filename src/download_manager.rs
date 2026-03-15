use std::fs::File;
use std::io::Write;
use std::path::Path;
use reqwest::blocking::get;
use std::os::windows::fs::OpenOptionsExt;
use std::fs::OpenOptions;
use crate::progress_bar::ProgressBar;
use std::io::Read;
use crate::models::{DownloadConfig, DownloadItem, DownloadStatus};
use crate::error::Result;
use crate::dprintln;



const CHUNK_SIZE_IN_KB: usize = 8;

pub fn download_file(url: &str, destination: &str) -> Result<(u64, Option<u64>)> {
    let mut start_byte = get_resume_start_byte(destination);
    
    let response = perform_http_request(url, start_byte)?;
    
    // Handle 416 Range Not Satisfiable (file already complete)
    if response.status() == 416 {
        dprintln!("File {} already complete ({} bytes)", destination, start_byte);
        return Ok((start_byte, Some(start_byte)));
    }
    
    // Check for successful response
    if !response.status().is_success() && response.status() != 206 {
        return Err(crate::error::DowmanError::Other(format!("File {} | HTTP error: {}", destination, response.status())));
    }

    if start_byte > 0 && response.status() == 200 {
        dprintln!("Server ignored Range header, restarting download from byte 0");
        start_byte = 0;
    }

    let file = prepare_destination_file(destination, start_byte)?;
    
    let expected_total_size = response.content_length().map(|len| len + start_byte);
    
    download_chunked(response, file, start_byte, expected_total_size, destination)
}

fn get_resume_start_byte(destination: &str) -> u64 {
    if Path::new(destination).exists()
        && let Ok(metadata) = std::fs::metadata(destination) {
            let start = metadata.len();
            dprintln!("Resuming download from byte {}", start);
            return start;
        }
    0
}

fn perform_http_request(url: &str, start_byte: u64) -> Result<reqwest::blocking::Response> {
    if start_byte > 0 {
        let range_header = format!("bytes={}-", start_byte);
        dprintln!("Using Range header: {}", range_header);
        let client = reqwest::blocking::Client::new();
        client.get(url).header("Range", range_header).send()
            .map_err(|e| crate::error::DowmanError::Other(format!("Failed to fetch URL: {}", e)))
    } else {
        get(url).map_err(|e| crate::error::DowmanError::Other(format!("Failed to fetch URL: {}", e)))
    }
}

fn prepare_destination_file(destination: &str, start_byte: u64) -> Result<File> {
    if let Some(parent) = Path::new(destination).parent()
        && !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| crate::error::DowmanError::Other(format!("Failed to create directories: {}", e)))?;
        }

    if start_byte > 0 {
        OpenOptions::new()
            .create(true)
            .append(true)
            .share_mode(0x07)   
            .open(destination)
            .map_err(|e| crate::error::DowmanError::Other(format!("Failed to open file:{} for resume: {}", destination, e)))
    } else {
        File::create(destination).map_err(|e| crate::error::DowmanError::Other(format!("Failed to create file:{}: {}", destination, e)))
    }
}

fn download_chunked(
    mut response: reqwest::blocking::Response, 
    mut file: File, 
    start_byte: u64, 
    expected_size: Option<u64>,
    destination: &str
) -> Result<(u64, Option<u64>)> {
    let display_total_size = expected_size.unwrap_or(0);

    let pbar = ProgressBar::new(
        display_total_size as usize,
        format!("Downloading {}", destination),
        Some(40),
    );

    pbar.start();
    if start_byte > 0 {
        pbar.update(start_byte as usize);
    }

    let mut downloaded: u64 = start_byte;
    let mut buffer = [0u8; CHUNK_SIZE_IN_KB * 1024];

    loop {
        let bytes_read = response
            .read(&mut buffer)
            .map_err(|e| crate::error::DowmanError::Other(format!("Failed reading response: {}", e)))?;

        if bytes_read == 0 {
            break;
        }

        file.write_all(&buffer[..bytes_read])
            .map_err(|e| crate::error::DowmanError::Other(format!("Failed to write file: {}", e)))?;

        downloaded += bytes_read as u64;
        pbar.update(bytes_read);
    }

    pbar.finish(false);
    let bytes_downloaded = downloaded - start_byte;
    dprintln!("Downloaded {} bytes (total: {})", bytes_downloaded, downloaded);
    
    Ok((downloaded, expected_size))
}
pub fn load_tracker(file_path: &str) -> Result<DownloadConfig> {
    use std::fs::OpenOptions;
    
    let mut options = OpenOptions::new();
    options.read(true);
    options.share_mode(0); // No sharing - exclusive access while using
    
    let mut file = options.open(file_path).map_err(|e| crate::error::DowmanError::Other(format!("Failed to open tracker file:{}: {}", file_path, e)))?;
    let mut content = String::new();
    use std::io::Read;
    file.read_to_string(&mut content).map_err(|e| crate::error::DowmanError::Other(format!("Failed to read tracker file:{}: {}", file_path, e)))?;
    
    serde_json::from_str(&content)
        .map_err(|e| crate::error::DowmanError::Other(format!("Failed to parse JSON: {}", e)))
}

pub fn save_tracker(config: &DownloadConfig, file_path: &str) -> Result<()> {
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| crate::error::DowmanError::Other(format!("Failed to serialize config: {}", e)))?;
    
    // Create parent directories if they don't exist
    if let Some(parent) = Path::new(file_path).parent()
        && !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| crate::error::DowmanError::Other(format!("Failed to create directories: {}", e)))?;
        }
    
    // Write the file with exclusive access
    use std::fs::OpenOptions;
    use std::io::Write;
    
    let mut options = OpenOptions::new();
    options.create(true).write(true).truncate(true);  // ← ADD TRUNCATE
    options.share_mode(0); // No sharing - exclusive access while using
    
    if let Ok(mut file) = options.open(file_path) {
        file.write_all(json.as_bytes())
            .map_err(|e| crate::error::DowmanError::Other(format!("Failed to write tracker file:{}: {}", file_path, e)))?;
    }
    
    Ok(())
}

pub fn initialize_tracker(ids: Vec<String>, file_path: &str) -> Result<DownloadConfig> {
    let downloads = ids
        .into_iter()
        .enumerate()
        .map(|(i, id)| DownloadItem {
            id: id.clone(),
            name: format!("Download {}", i + 1),
            url: String::new(),
            destination: format!("downloads/{}.txt", id),
            status_info: DownloadStatus::NotStarted,
        })
        .collect();

    let config = DownloadConfig {
        name: "Dowman Download Tracker".to_string(),
        downloads,
    };

    save_tracker(&config, file_path)?;
    Ok(config)
}
