use clean_dowman::{SmartManager, Verbosity, init_file_logging, vprintln, dprintln, set_verbosity};
use clap::Parser;
use log::LevelFilter;
use std::fs;
use std::collections::HashMap;
use std::env;

#[derive(Parser)]
#[command(name = "dowman")]
#[command(about = "A smart download manager with real-time tracking")]
struct Cli {
    /// Print literally every step
    #[arg(long = "verbose")]
    verbose: bool,
    
    /// Print major things only
    #[arg(long = "debug")]
    debug: bool,
    
    /// Don't print anything (including progress bars)
    #[arg(long = "no-console")]
    no_console: bool,
    
    /// Custom tracking model file path
    #[arg(long = "tracking-model-file")]
    tracking_model_file: Option<String>,
    
    /// Custom registry key path (only used on first run)
    #[arg(long = "registry-key-path")]
    registry_key_path: Option<String>,
    
    /// Log file path for detailed logging
    #[arg(long = "log-file")]
    log_file: Option<String>,

    /// get download information from json file
    #[arg(long = "required-downloads-jfile")]
    required_downloads_jfile: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Initialize logging if log file is provided
    if let Some(log_file) = &cli.log_file {
        let log_level = if cli.verbose {
            LevelFilter::Debug
        } else if cli.debug {
            LevelFilter::Info
        } else {
            LevelFilter::Warn
        };
        
        init_file_logging(log_file, &log_level.to_string())
            .map_err(|e| format!("Logger init failed: {}", e))?;
    }
    
    let verbosity = if cli.no_console {
        Verbosity::NoConsole
    } else if cli.verbose {
        Verbosity::Verbose
    } else if cli.debug {
        Verbosity::Debug
    } else {
        Verbosity::Normal
    };
    
    set_verbosity!(match verbosity {
        Verbosity::NoConsole => 0,
        Verbosity::Verbose => 1,
        Verbosity::Debug => 2,
        Verbosity::Normal => 3,
    });
    
    run_download_manager(verbosity, cli.tracking_model_file, cli.registry_key_path, cli.required_downloads_jfile)?;
    Ok(())
}

fn run_download_manager(verbosity: Verbosity, tracking_file: Option<String>, registry_key_path: Option<String>, required_downloads_jfile: Option<String>) -> clean_dowman::error::Result<()> {
    vprintln!("VERBOSE MODE: Printing literally every step");
    dprintln!("DEBUG MODE: Printing major things only");
    vprintln!("Dowman Download Manager");
    dprintln!("Dowman Download Manager");
    
    // Create a new smart manager with custom tracking file
    let mut manager = SmartManager::new(
        String::from("Dowman Download Manager"),
        tracking_file.or_else(|| Some("my_downloads.json".to_string())),
        registry_key_path,
    );

    // Check if this is first run and print appropriate message
    if manager.is_first_run() {
        vprintln!("VERBOSE: First run detected - creating tracking file and registry entries");
        dprintln!("DEBUG: First run detected - initializing");
        vprintln!("First run detected - initializing download manager");
        dprintln!("First run detected - initializing download manager");
    } else {
        vprintln!("VERBOSE: Existing tracking file found - loading previous state");
        dprintln!("DEBUG: Loading existing tracking data");
        vprintln!("Loading existing download data");
        dprintln!("Loading existing download data");
    }

    // Load downloads from hashmap
    let downloads = get_downloads_from_hashmap(verbosity, required_downloads_jfile);
    
    for (id, (name, url, destination)) in downloads {
        manager.add_download(
            id,
            name,
            url,
            destination,
        );
    }

    // Start downloads (will check tracking file and download what's needed)
    manager.start_downloads_with_verbosity(verbosity)?;

    // Print final status
    vprintln!("\nFinal Status:");
    dprintln!("\nFinal Status:");
    let status = manager.get_status();
    for download in &status.downloads {
        vprintln!("  {}: {:?}", download.name, download.status_info);
        dprintln!("  {}: {:?}", download.name, download.status_info);
    }

    Ok(())
}


fn expand_env_vars(path: &str) -> String {
    // Handle Windows-style environment variables like %TEMP%
    let mut result = path.to_string();
    
    // Find all %VAR% patterns and replace them
    let mut start = 0;
    while let Some(var_start) = result[start..].find('%') {
        let var_start = start + var_start;
        if let Some(var_end) = result[var_start + 1..].find('%') {
            let var_end = var_start + 1 + var_end;
            let var_name = &result[var_start + 1..var_end];
            
            if let Ok(var_value) = env::var(var_name) {
                result.replace_range(var_start..=var_end, &var_value);
                start = var_start + var_value.len();
            } else {
                start = var_end + 1;
            }
        } else {
            break;
        }
    }
    
    // Normalize path separators to use forward slashes consistently
    result.replace('\\', "/")
}

fn get_downloads_from_hashmap(_verbosity: Verbosity, required_downloads_jfile: Option<String>) -> std::collections::HashMap<String, (String, String, String)> {
    
    if let Some(jfile) = required_downloads_jfile {
        match fs::read_to_string(&jfile) {
            Ok(content) => {
                match serde_json::from_str::<HashMap<String, (String, String, String)>>(&content) {
                    Ok(mut map) => {
                        // Expand environment variables in destination paths
                        for (_, (_name, _url, destination)) in map.iter_mut() {
                            *destination = expand_env_vars(destination);
                        }
                        return map;
                    },
                    Err(e) => dprintln!("Error parsing JSON: {}. Using defaults.", e),
                }
            }
            Err(e) => dprintln!("Error reading file {}: {}. Using defaults.", jfile, e),
        }
    }


    let mut downloads = std::collections::HashMap::new();
    
    // Add downloads to hashmap: id -> (name, url, destination)
    downloads.insert(
        "file1".to_string(),
        (
            "Rust Logo".to_string(),
            "https://www.rust-lang.org/static/images/rust-logo-256x256.png".to_string(),
            "downloads/rust-logo.png".to_string(),
        ),
    );
    
    downloads.insert(
        "file2".to_string(),
        (
            "Test File".to_string(),
            "https://httpbin.org/json".to_string(),
            "downloads/test.json".to_string(),
        ),
    );
    
    downloads.insert(
        "file3".to_string(),
        (
            "Example PDF".to_string(),
            "https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf".to_string(),
            "downloads/dummy.pdf".to_string(),
        ),
    );
    
    downloads
}
