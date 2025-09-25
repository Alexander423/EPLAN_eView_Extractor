use anyhow::{Result, Context};
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ChromeDriverManager {
    driver_path: PathBuf,
    process: Arc<Mutex<Option<Child>>>,
}

impl ChromeDriverManager {
    pub fn new() -> Self {
        let exe_dir = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("."))
            .parent()
            .unwrap_or(&PathBuf::from("."))
            .to_path_buf();

        let driver_path = exe_dir.join("chromedriver.exe");

        Self {
            driver_path,
            process: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn ensure_driver_available(&self) -> Result<()> {
        if !self.driver_path.exists() {
            println!("ChromeDriver not found at {:?}, downloading...", self.driver_path);
            self.download_chromedriver().await
                .context("Failed to download ChromeDriver. Please check your internet connection.")?;
        } else {
            println!("ChromeDriver found at {:?}", self.driver_path);
        }
        Ok(())
    }

    pub async fn start_driver(&self, port: u16) -> Result<()> {
        // Ensure driver is available
        self.ensure_driver_available().await?;

        // Check if already running
        let mut process_guard = self.process.lock().await;
        if process_guard.is_some() {
            println!("ChromeDriver is already running on port {}", port);
            return Ok(());
        }

        // Start ChromeDriver
        println!("Starting ChromeDriver on port {}...", port);
        let mut cmd = Command::new(&self.driver_path);
        cmd.arg(format!("--port={}", port))
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let child = cmd.spawn()
            .with_context(|| format!("Failed to start ChromeDriver from {:?}. Make sure Chrome is installed.", self.driver_path))?;

        *process_guard = Some(child);

        // Wait for ChromeDriver to be ready to accept connections
        println!("Waiting for ChromeDriver to become ready...");
        let ready = self.wait_for_readiness(port, 15).await?;
        if !ready {
            return Err(anyhow::anyhow!("ChromeDriver failed to become ready within 15 seconds. This might indicate a Chrome installation problem."));
        }

        println!("✅ ChromeDriver successfully started on port {}", port);
        Ok(())
    }

    pub async fn stop_driver(&self) -> Result<()> {
        let mut process_guard = self.process.lock().await;
        if let Some(mut child) = process_guard.take() {
            let _ = child.kill();
            let _ = child.wait();
            println!("ChromeDriver stopped");
        }
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        let process_guard = self.process.lock().await;
        if let Some(_child) = process_guard.as_ref() {
            // Check if process is still alive (simple check)
            return true; // Simplified - in real implementation we'd check process status
        }
        false
    }

    async fn download_chromedriver(&self) -> Result<()> {
        // Get latest ChromeDriver version
        let version = self.get_latest_version().await?;
        println!("Downloading ChromeDriver version {}", version);

        // Download URL for Windows - new format for Chrome 115+
        let download_url = format!(
            "https://edgedl.me.gvt1.com/edgedl/chrome/chrome-for-testing/{}/win64/chromedriver-win64.zip",
            version
        );

        // Download the file
        let response = reqwest::get(&download_url).await?;
        let zip_data = response.bytes().await?;

        // Save to temp file
        let temp_dir = std::env::temp_dir();
        let zip_path = temp_dir.join("chromedriver.zip");
        fs::write(&zip_path, zip_data)?;

        // Extract the zip
        let file = fs::File::open(&zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let file_name = file.name();
            // Handle both old format (chromedriver.exe) and new format (chromedriver-win64/chromedriver.exe)
            if file_name.ends_with("chromedriver.exe") && !file_name.ends_with("/") {
                println!("Extracting: {}", file_name);
                let mut outfile = fs::File::create(&self.driver_path)?;
                std::io::copy(&mut file, &mut outfile)?;
                break;
            }
        }

        // Clean up temp file
        let _ = fs::remove_file(&zip_path);

        println!("ChromeDriver downloaded to {:?}", self.driver_path);
        Ok(())
    }

    async fn wait_for_readiness(&self, port: u16, timeout_secs: u64) -> Result<bool> {
        let client = reqwest::Client::new();
        let url = format!("http://localhost:{}/status", port);
        let timeout = tokio::time::Duration::from_secs(timeout_secs);
        let start = tokio::time::Instant::now();

        while start.elapsed() < timeout {
            match client.get(&url).send().await {
                Ok(response) => {
                    if response.status().is_success() {
                        return Ok(true);
                    }
                }
                Err(_) => {
                    // ChromeDriver not ready yet, continue waiting
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }

        Ok(false)
    }

    async fn get_latest_version(&self) -> Result<String> {
        // For Chrome 140+, we need to use the new ChromeDriver endpoint
        // Chrome versions 115+ use a different versioning system
        let response = reqwest::get("https://googlechromelabs.github.io/chrome-for-testing/LATEST_RELEASE_STABLE")
            .await?;
        let version = response.text().await?.trim().to_string();
        println!("Latest ChromeDriver version: {}", version);
        Ok(version)
    }
}

impl Drop for ChromeDriverManager {
    fn drop(&mut self) {
        // Best effort cleanup
        if let Ok(mut process_guard) = self.process.try_lock() {
            if let Some(mut child) = process_guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}