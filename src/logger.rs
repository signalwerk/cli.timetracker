use std::path::PathBuf;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use anyhow::Result;
use chrono::{DateTime, Utc};

pub struct Logger {
    log_path: PathBuf,
}

impl Logger {
    pub fn new() -> Result<Self> {
        let log_path = Self::get_log_path()?;
        Ok(Self { log_path })
    }

    fn get_log_path() -> Result<PathBuf> {
        // Check if we're in development mode (if Cargo.toml exists in current directory)
        let current_dir = std::env::current_dir()?;
        let cargo_toml = current_dir.join("Cargo.toml");
        
        if cargo_toml.exists() {
            // Development mode - use current directory
            Ok(current_dir.join("timetracker.log"))
        } else {
            // Production mode - use home directory
            match dirs::home_dir() {
                Some(home) => Ok(home.join(".timetracker.log")),
                None => {
                    // Fallback to current directory if home directory is not found
                    Ok(current_dir.join("timetracker.log"))
                }
            }
        }
    }

    pub async fn log(&self, message: &str) -> Result<()> {
        let timestamp: DateTime<Utc> = Utc::now();
        let log_entry = format!("[{}] {}\n", timestamp.format("%Y-%m-%d %H:%M:%S UTC"), message);
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await?;
        
        file.write_all(log_entry.as_bytes()).await?;
        file.flush().await?;
        
        Ok(())
    }

    pub fn get_current_log_path(&self) -> &PathBuf {
        &self.log_path
    }
} 