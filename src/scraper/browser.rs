use anyhow::{Result, Context};
use thirtyfour::prelude::*;
use tokio::time::{sleep, Duration};

pub struct BrowserDriver {
    driver: WebDriver,
}

impl BrowserDriver {
    pub async fn new(headless: bool) -> Result<Self> {
        println!("DEBUG: BrowserDriver::new() - Starting with headless={}", headless);

        // Create Chrome capabilities with proper arguments
        let mut caps = DesiredCapabilities::chrome();

        // Add Chrome arguments for better stability
        let mut chrome_args = vec![
            "--no-sandbox".to_string(),
            "--disable-dev-shm-usage".to_string(),
            "--disable-gpu".to_string(),
            "--disable-web-security".to_string(),
            "--disable-features=VizDisplayCompositor".to_string(),
            "--remote-debugging-port=9222".to_string(),
            "--window-size=1920,1080".to_string(),
        ];

        if headless {
            chrome_args.push("--headless".to_string());
        }

        // Add Chrome arguments to capabilities
        let args_count = chrome_args.len();
        for arg in chrome_args {
            caps.add_arg(&arg)?;
        }

        println!("DEBUG: BrowserDriver::new() - Chrome capabilities created with {} args", args_count);

        // Connect to ChromeDriver with reduced retry logic
        let mut last_error = None;
        for attempt in 1..=3 {
            println!("DEBUG: BrowserDriver::new() - Connection attempt {}/3", attempt);
            match WebDriver::new("http://localhost:9516", caps.clone()).await {
                Ok(driver) => {
                    println!("DEBUG: BrowserDriver::new() - Successfully connected to ChromeDriver");
                    return Ok(Self { driver });
                }
                Err(e) => {
                    println!("DEBUG: BrowserDriver::new() - Attempt {} failed: {}", attempt, e);
                    last_error = Some(e);
                    if attempt < 3 {
                        // Short delay between attempts
                        let delay = Duration::from_millis(1000);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
            .context("Failed to connect to ChromeDriver after 3 attempts. ChromeDriver should have been started automatically on port 9516")
    }

    pub async fn navigate(&self, url: &str) -> Result<()> {
        self.driver.goto(url).await?;
        Ok(())
    }

    pub async fn find_element(&self, selector: By) -> Result<WebElement> {
        self.driver.find(selector).await
            .context("Element not found")
    }

    pub async fn find_elements(&self, selector: By) -> Result<Vec<WebElement>> {
        Ok(self.driver.find_all(selector).await?)
    }

    pub async fn wait_for_element(&self, selector: By, timeout_secs: u64) -> Result<WebElement> {
        let timeout = Duration::from_secs(timeout_secs);
        let start = std::time::Instant::now();

        loop {
            if let Ok(element) = self.driver.find(selector.clone()).await {
                return Ok(element);
            }

            if start.elapsed() > timeout {
                return Err(anyhow::anyhow!("Timeout waiting for element"));
            }

            sleep(Duration::from_millis(500)).await;
        }
    }

    pub async fn click_element(&self, element: &WebElement) -> Result<()> {
        element.click().await?;
        Ok(())
    }

    pub async fn send_keys(&self, element: &WebElement, text: &str) -> Result<()> {
        element.clear().await?;
        element.send_keys(text).await?;
        Ok(())
    }

    pub async fn get_page_source(&self) -> Result<String> {
        Ok(self.driver.source().await?)
    }

    pub async fn get_current_url(&self) -> Result<String> {
        Ok(self.driver.current_url().await?.to_string())
    }

    pub async fn execute_script(&self, script: &str, args: Vec<WebElement>) -> Result<()> {
        // Convert WebElement to serde_json::Value
        let json_args: Vec<serde_json::Value> = args.into_iter()
            .map(|el| serde_json::json!(el))
            .collect();

        self.driver.execute(script, json_args).await?;
        Ok(())
    }

    pub async fn execute_script_and_get_value(&self, script: &str, args: Vec<WebElement>) -> Result<serde_json::Value> {
        // Convert WebElements to JSON values for the script execution
        let json_args: Vec<serde_json::Value> = args.into_iter()
            .map(|el| serde_json::json!(el))
            .collect();

        match self.driver.execute(script, json_args).await {
            Ok(value) => Ok(value.json().clone()),
            Err(e) => Err(anyhow::anyhow!("Script execution failed: {}", e)),
        }
    }

    pub async fn quit(&self) -> Result<()> {
        // Clone the driver to move it into quit()
        let driver_clone = self.driver.clone();
        driver_clone.quit().await?;
        Ok(())
    }

    // Helper methods for Microsoft login
    pub async fn find_microsoft_button(&self) -> Result<Option<WebElement>> {
        let selectors = vec![
            By::XPath("//*[contains(text(), 'Microsoft')]"),
            By::XPath("//*[contains(text(), 'microsoft')]"),
            By::XPath("//*[contains(@title, 'Microsoft')]"),
        ];

        for selector in selectors {
            if let Ok(elements) = self.find_elements(selector).await {
                for element in elements {
                    if element.is_displayed().await.unwrap_or(false)
                        && element.is_enabled().await.unwrap_or(false) {
                        return Ok(Some(element));
                    }
                }
            }
        }

        Ok(None)
    }

    pub async fn find_email_field(&self) -> Result<Option<WebElement>> {
        let selectors = vec![
            By::Css("input[type='email']"),
            By::Css("input[name='loginfmt']"),
            By::Css("input[id='i0116']"),
        ];

        for selector in selectors {
            if let Ok(element) = self.find_element(selector).await {
                if element.is_displayed().await.unwrap_or(false) {
                    return Ok(Some(element));
                }
            }
        }

        Ok(None)
    }

    pub async fn find_password_field(&self) -> Result<Option<WebElement>> {
        let selectors = vec![
            By::Css("input[type='password']"),
            By::Css("input[name='passwd']"),
            By::Css("input[id='i0118']"),
        ];

        for selector in selectors {
            if let Ok(element) = self.find_element(selector).await {
                if element.is_displayed().await.unwrap_or(false) {
                    return Ok(Some(element));
                }
            }
        }

        Ok(None)
    }

    pub async fn find_submit_button(&self) -> Result<Option<WebElement>> {
        let selectors = vec![
            By::Css("input[type='submit']"),
            By::Css("input[id='idSIButton9']"),
            By::Css("button[type='submit']"),
        ];

        for selector in selectors {
            if let Ok(element) = self.find_element(selector).await {
                if element.is_displayed().await.unwrap_or(false)
                    && element.is_enabled().await.unwrap_or(false) {
                    return Ok(Some(element));
                }
            }
        }

        Ok(None)
    }
}