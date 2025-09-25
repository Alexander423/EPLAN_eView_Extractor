pub mod browser;
pub mod extractor;

use anyhow::Result;
use crate::models::{PlcTable, PlcEntry, PlcDataType};
use crate::chromedriver_manager::ChromeDriverManager;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct ScraperEngine {
    browser: browser::BrowserDriver,
    config: ScraperConfig,
    logger: Arc<Mutex<Box<dyn Logger>>>,
    chromedriver_manager: Arc<ChromeDriverManager>,
    extracted_table: Option<PlcTable>,
}

#[derive(Debug, Clone)]
pub struct ScraperConfig {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub project_number: String,
    pub headless: bool,
}

pub trait Logger: Send + Sync {
    fn log(&self, message: String, level: LogLevel);
}

#[derive(Debug, Clone)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Success,
    Debug,
}

impl ScraperEngine {
    pub async fn new(config: ScraperConfig, logger: Arc<Mutex<Box<dyn Logger>>>, chromedriver_manager: Arc<ChromeDriverManager>) -> Result<Self> {
        println!("DEBUG: ScraperEngine::new() - Starting");

        // Start ChromeDriver first
        println!("DEBUG: ScraperEngine::new() - Starting ChromeDriver on port 9516");
        chromedriver_manager.start_driver(9516).await
            .map_err(|e| anyhow::anyhow!("Failed to start ChromeDriver: {}", e))?;

        // Wait a bit for ChromeDriver to fully start
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

        println!("DEBUG: ScraperEngine::new() - About to create BrowserDriver");
        let browser = browser::BrowserDriver::new(config.headless).await?;

        println!("DEBUG: ScraperEngine::new() - BrowserDriver created successfully");

        Ok(Self {
            browser,
            config,
            logger,
            chromedriver_manager,
            extracted_table: None,
        })
    }

    pub async fn run_extraction(&mut self) -> Result<PlcTable> {
        self.log("🚀 Starting eVIEW extraction process...".to_string(), LogLevel::Info).await;

        // Step 1: Navigate to base URL
        self.log("📍 Step 1/6: Navigating to eVIEW...".to_string(), LogLevel::Info).await;
        match self.browser.navigate(&self.config.base_url).await {
            Ok(_) => {
                self.log(format!("✅ Successfully navigated to {}", self.config.base_url), LogLevel::Success).await;
            }
            Err(e) => {
                self.log(format!("❌ Failed to navigate to eVIEW: {}", e), LogLevel::Error).await;
                return Err(anyhow::anyhow!("Navigation to eVIEW failed: {}", e));
            }
        }

        // Step 2: Handle Microsoft login
        self.log("📍 Step 2/6: Handling Microsoft login...".to_string(), LogLevel::Info).await;
        match self.click_microsoft_login().await {
            Ok(_) => {
                self.log("✅ Microsoft login button clicked successfully".to_string(), LogLevel::Success).await;
            }
            Err(e) => {
                self.log(format!("❌ Failed to click Microsoft login: {}", e), LogLevel::Error).await;
                return Err(anyhow::anyhow!("Microsoft login button click failed: {}", e));
            }
        }

        self.log("🔐 Performing Microsoft SSO login...".to_string(), LogLevel::Info).await;
        match self.perform_login().await {
            Ok(_) => {
                self.log("✅ Microsoft SSO login completed successfully".to_string(), LogLevel::Success).await;
            }
            Err(e) => {
                self.log(format!("❌ Microsoft login process failed: {}", e), LogLevel::Error).await;
                return Err(anyhow::anyhow!("Microsoft login failed: {}", e));
            }
        }

        // Step 3: Open the specific project
        self.log("📍 Step 3/6: Opening project...".to_string(), LogLevel::Info).await;
        match self.open_project().await {
            Ok(_) => {
                self.log(format!("✅ Project '{}' opened successfully", self.config.project_number), LogLevel::Success).await;
            }
            Err(e) => {
                self.log(format!("❌ Failed to open project '{}': {}", self.config.project_number, e), LogLevel::Error).await;
                return Err(anyhow::anyhow!("Project opening failed: {}", e));
            }
        }

        // Step 4: Switch to list view
        self.log("📍 Step 4/6: Switching to list view...".to_string(), LogLevel::Info).await;
        match self.switch_to_list_view().await {
            Ok(_) => {
                self.log("✅ Successfully switched to list view".to_string(), LogLevel::Success).await;
            }
            Err(e) => {
                self.log(format!("❌ Failed to switch to list view: {}", e), LogLevel::Error).await;
                return Err(anyhow::anyhow!("List view switch failed: {}", e));
            }
        }

        // Step 5: Extract the tables
        self.log("📍 Step 5/6: Extracting SPS tables...".to_string(), LogLevel::Info).await;
        match self.extract_tables().await {
            Ok(success) => {
                if success {
                    self.log("✅ SPS table extraction completed successfully!".to_string(), LogLevel::Success).await;
                } else {
                    self.log("⚠️ SPS table extraction completed but found no tables".to_string(), LogLevel::Warning).await;
                }
            }
            Err(e) => {
                self.log(format!("❌ Table extraction failed: {}", e), LogLevel::Error).await;
                return Err(anyhow::anyhow!("Table extraction failed: {}", e));
            }
        }

        // Return the extracted table (or an empty one if extraction failed)
        let table = self.extracted_table.take().unwrap_or_else(|| PlcTable::new(self.config.project_number.clone()));
        self.log(format!("✅ Final result: {} entries extracted", table.entries.len()), LogLevel::Success).await;

        // Step 6: Final completion
        self.log("📍 Step 6/6: Finalizing extraction...".to_string(), LogLevel::Info).await;
        self.log(format!("🎉 Extraction completed successfully! Found {} entries", table.entries.len()), LogLevel::Success).await;

        Ok(table)
    }

    async fn log(&self, message: String, level: LogLevel) {
        let logger = self.logger.lock().await;
        logger.log(message, level);
    }

    async fn click_microsoft_login(&mut self) -> Result<()> {
        self.log("Looking for Microsoft login button".to_string(), LogLevel::Info).await;

        // Try multiple times to find the Microsoft login button (Python: 15 attempts)
        for attempt in 1..=15 {
            self.log(format!("Looking for Microsoft button... [{}/15]", attempt), LogLevel::Info).await;

            // Find all buttons first (debugging)
            if let Ok(all_buttons) = self.browser.find_elements(thirtyfour::By::Tag("button")).await {
                self.log(format!("Found buttons: {}", all_buttons.len()), LogLevel::Debug).await;

                // Log first few buttons for debugging
                for (i, btn) in all_buttons.iter().take(5).enumerate() {
                    if let Ok(is_displayed) = btn.is_displayed().await {
                        if is_displayed {
                            let text = btn.text().await.unwrap_or_default();
                            let value = btn.attr("value").await.unwrap_or(None).unwrap_or_default();
                            let class = btn.attr("class").await.unwrap_or(None).unwrap_or_default();
                            self.log(format!("Button {}: '{}' | Value: '{}' | Class: '{}'", i, text, value, class), LogLevel::Debug).await;
                        }
                    }
                }
            }

            // Find all elements containing 'Microsoft' text
            let microsoft_selectors = vec![
                "//*[contains(text(), 'Microsoft') or contains(text(), 'microsoft') or contains(@title, 'Microsoft')]"
            ];

            for selector in microsoft_selectors {
                if let Ok(elements) = self.browser.find_elements(thirtyfour::By::XPath(selector)).await {
                    for elem in elements {
                        match (elem.is_displayed().await, elem.is_enabled().await) {
                            (Ok(true), Ok(true)) => {
                                if let Ok(()) = elem.click().await {
                                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                                    // Check if we navigated to Microsoft login
                                    if let Ok(url) = self.browser.get_current_url().await {
                                        if url.contains("login.microsoft") {
                                            self.log("Successfully clicked Microsoft login button".to_string(), LogLevel::Success).await;
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                            _ => continue,
                        }
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        Err(anyhow::anyhow!("Could not find Microsoft login button after 15 attempts"))
    }

    async fn perform_login(&mut self) -> Result<()> {
        self.log("Waiting for Microsoft email field...".to_string(), LogLevel::Info).await;

        // Email field selectors from Python
        let email_selectors = vec![
            "input[type='email']",
            "input[name='loginfmt']",
            "input[id='i0116']",
            "input[id='email']",
            "input[placeholder*='Email']",
            "input[placeholder*='E-Mail']",
            "input[name='username']",
        ];

        // Find email field with retry logic
        let mut email_field = None;
        for attempt in 1..=15 {
            self.log(format!("Waiting for email field... [{}/15]", attempt), LogLevel::Debug).await;

            for selector in &email_selectors {
                if let Ok(field) = self.browser.find_element(thirtyfour::By::Css(*selector)).await {
                    if field.is_displayed().await.unwrap_or(false) {
                        self.log(format!("Email field found with selector: {}", selector), LogLevel::Debug).await;
                        email_field = Some(field);
                        break;
                    }
                }
            }
            if email_field.is_some() { break; }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        let email_field = email_field.ok_or_else(|| anyhow::anyhow!("Email field not found"))?;

        // Enter email
        self.log("Type in email...".to_string(), LogLevel::Info).await;
        email_field.clear().await.map_err(|_| anyhow::anyhow!("Unable to clear email field"))?;
        email_field.send_keys(&self.config.username).await.map_err(|_| anyhow::anyhow!("Unable to type in email"))?;

        // Click Next button
        self.log("Looking for 'Next' button...".to_string(), LogLevel::Info).await;
        let next_button_selectors = vec![
            "input[type='submit']",
            "input[id='idSIButton9']",
            "button[type='submit']",
            "input[value='Next']",
            "input[value='Weiter']",
            "button[id='idSIButton9']",
        ];

        let mut next_clicked = false;
        for selector in &next_button_selectors {
            if let Ok(next_button) = self.browser.find_element(thirtyfour::By::Css(*selector)).await {
                if next_button.is_displayed().await.unwrap_or(false) && next_button.is_enabled().await.unwrap_or(false) {
                    next_button.click().await?;
                    self.log(format!("'Next' button clicked with selector: {}", selector), LogLevel::Debug).await;
                    next_clicked = true;
                    break;
                }
            }
        }

        if !next_clicked {
            // Alternative: Press Enter
            email_field.send_keys(thirtyfour::Key::Return).await?;
            self.log("Submit-button pressed instead of Next-button".to_string(), LogLevel::Debug).await;
        }

        // Wait for password page
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Password field logic
        self.log("Looking for password field...".to_string(), LogLevel::Info).await;
        let password_selectors = vec![
            "input[type='password']",
            "input[name='passwd']",
            "input[id='i0118']",
            "input[id='passwordInput']",
            "input[placeholder*='Password']",
            "input[placeholder*='Passwort']",
        ];

        let mut password_field = None;
        for attempt in 1..=15 {
            for selector in &password_selectors {
                if let Ok(field) = self.browser.find_element(thirtyfour::By::Css(*selector)).await {
                    if field.is_displayed().await.unwrap_or(false) {
                        self.log(format!("Password field found with selector: {}", selector), LogLevel::Debug).await;
                        password_field = Some(field);
                        break;
                    }
                }
            }
            if password_field.is_some() { break; }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            self.log(format!("Waiting for password field... [{}/15]", attempt), LogLevel::Debug).await;
        }

        if let Some(password_field) = password_field {
            self.log("Inserting password...".to_string(), LogLevel::Info).await;
            password_field.clear().await?;
            password_field.send_keys(&self.config.password).await?;

            // Click Sign-In button
            self.log("Looking for 'Sign-In' button".to_string(), LogLevel::Info).await;
            let signin_button_selectors = vec![
                "input[type='submit']",
                "input[id='idSIButton9']",
                "button[type='submit']",
                "input[value='Sign in']",
                "input[value='Anmelden']",
                "button[id='idSIButton9']",
            ];

            let mut signin_clicked = false;
            for selector in &signin_button_selectors {
                if let Ok(signin_button) = self.browser.find_element(thirtyfour::By::Css(*selector)).await {
                    if signin_button.is_displayed().await.unwrap_or(false) && signin_button.is_enabled().await.unwrap_or(false) {
                        signin_button.click().await?;
                        self.log(format!("'Sign-In' button clicked with selector: {}", selector), LogLevel::Debug).await;
                        signin_clicked = true;
                        break;
                    }
                }
            }

            if !signin_clicked {
                password_field.send_keys(thirtyfour::Key::Return).await?;
                self.log("Submit pressed instead of 'Log-In' click".to_string(), LogLevel::Debug).await;
            }
        } else {
            self.log("Password field not found - maybe 'Single Sign-On' active".to_string(), LogLevel::Warning).await;
        }

        // Handle "Stay signed in?" dialog
        for attempt in 1..=15 {
            self.log(format!("Trying to click on 'Yes' button... [{}/15]", attempt), LogLevel::Debug).await;

            let stay_signed_selectors = vec![
                "input[id='idSIButton9']",
                "input[value='Yes']",
                "input[value='Ja']",
                "button[id='idSIButton9']",
            ];

            let mut clicked = false;
            for selector in &stay_signed_selectors {
                if let Ok(button) = self.browser.find_element(thirtyfour::By::Css(*selector)).await {
                    if button.is_displayed().await.unwrap_or(false) && button.is_enabled().await.unwrap_or(false) {
                        button.click().await?;
                        self.log("'Stay logged in' dialogue answered with 'Yes'".to_string(), LogLevel::Debug).await;
                        clicked = true;
                        break;
                    }
                }
            }
            if clicked { break; }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        // Handle organization selection if multi-org dialog appears
        self.handle_organization_selection().await?;

        self.log("Waiting for return to EPLAN eVIEW...".to_string(), LogLevel::Info).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Check if login was successful
        let current_url = self.browser.get_current_url().await?;
        if !current_url.to_lowercase().contains("login") &&
           (current_url.contains(&self.config.base_url) || current_url.to_lowercase().contains("eview")) {
            self.log("Microsoft SSO login successful!".to_string(), LogLevel::Success).await;
            Ok(())
        } else {
            self.log(format!("Login status unclear. Current URL: {}", current_url), LogLevel::Warning).await;
            Err(anyhow::anyhow!("Login verification failed"))
        }
    }

    async fn handle_organization_selection(&mut self) -> Result<()> {
        self.log("Checking for organization selection dialog...".to_string(), LogLevel::Debug).await;

        // Check if we're on an organization selection page
        let current_url = self.browser.get_current_url().await?;
        if !current_url.to_lowercase().contains("organization") && !current_url.to_lowercase().contains("tenant") {
            self.log("No organization selection dialog detected".to_string(), LogLevel::Debug).await;
            return Ok(());
        }

        self.log("Organization selection dialog detected!".to_string(), LogLevel::Info).await;

        // Try to find and click the 3CON organization
        let organization_selectors = vec![
            "//div[contains(text(), '3CON Anlagenbau')]",
            "//div[contains(text(), '3con')]",
            "//div[contains(text(), '3CON')]",
            "//span[contains(text(), '3CON Anlagenbau')]",
            "//span[contains(text(), '3con')]",
            "//a[contains(text(), '3CON Anlagenbau')]",
            "//button[contains(text(), '3CON Anlagenbau')]",
            "//td[contains(text(), '3CON Anlagenbau')]",
        ];

        let mut organization_selected = false;
        for selector in &organization_selectors {
            self.log(format!("Trying selector: {}", selector), LogLevel::Debug).await;

            if let Ok(element) = self.browser.find_element(thirtyfour::By::XPath(*selector)).await {
                if element.is_displayed().await.unwrap_or(false) {
                    self.log("Found 3CON organization option, clicking...".to_string(), LogLevel::Info).await;
                    element.click().await?;
                    organization_selected = true;
                    break;
                }
            }
        }

        if !organization_selected {
            self.log("Could not find 3CON organization option, trying fallback detection...".to_string(), LogLevel::Warning).await;

            // Fallback: look for any clickable element containing "3con" or "3CON"
            if let Ok(elements) = self.browser.find_elements(thirtyfour::By::XPath("//*[contains(translate(text(), 'ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'abcdefghijklmnopqrstuvwxyz'), '3con')]")).await {
                for element in elements {
                    if element.is_displayed().await.unwrap_or(false) && element.is_enabled().await.unwrap_or(false) {
                        let text = element.text().await.unwrap_or_default();
                        self.log(format!("Found fallback organization option: '{}'", text), LogLevel::Info).await;
                        element.click().await?;
                        organization_selected = true;
                        break;
                    }
                }
            }
        }

        if organization_selected {
            self.log("Organization selection completed successfully".to_string(), LogLevel::Success).await;

            // Give it a moment to process
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        } else {
            self.log("No 3CON organization found, proceeding anyway...".to_string(), LogLevel::Warning).await;
        }

        Ok(())
    }

    async fn open_project(&mut self) -> Result<()> {
        self.log(format!("Navigating to project: {}", self.config.project_number), LogLevel::Info).await;

        // Wait for project overview
        self.log("Waiting for project overview...".to_string(), LogLevel::Info).await;
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        self.log(format!("Looking for project '{}' in the list...", self.config.project_number), LogLevel::Info).await;

        // Various ways the project could be displayed (from Python)
        let project_selectors = vec![
            format!("//td[contains(text(), '{}')]", self.config.project_number),
            format!("//span[contains(text(), '{}')]", self.config.project_number),
            format!("//div[contains(text(), '{}')]", self.config.project_number),
            format!("//a[contains(text(), '{}')]", self.config.project_number),
            format!("//tr[contains(., '{}')]", self.config.project_number),
            format!("//*[text()='{}']", self.config.project_number),
        ];

        let mut project_element = None;

        for xpath in &project_selectors {
            match self.browser.find_elements(thirtyfour::By::XPath(xpath)).await {
                Ok(elements) if !elements.is_empty() => {
                    project_element = Some(elements[0].clone());
                    self.log(format!("Project found with XPath: {}", xpath), LogLevel::Success).await;
                    break;
                }
                _ => {
                    // Try single element fallback
                    if let Ok(element) = self.browser.find_element(thirtyfour::By::XPath(xpath)).await {
                        project_element = Some(element);
                        self.log(format!("Project-element found with XPath: {}", xpath), LogLevel::Success).await;
                        break;
                    }
                }
            }
        }

        if project_element.is_none() {
            // List all table rows for debugging (first 10)
            if let Ok(all_rows) = self.browser.find_elements(thirtyfour::By::Tag("tr")).await {
                self.log(format!("Found table rows: {}", all_rows.len()), LogLevel::Debug).await;
                for (i, row) in all_rows.iter().take(10).enumerate() {
                    if let Ok(row_text) = row.text().await {
                        let truncated_text = if row_text.len() > 100 {
                            format!("{}...", &row_text[..100])
                        } else {
                            row_text
                        };
                        self.log(format!("Row {}: {}", i, truncated_text), LogLevel::Debug).await;
                    }
                }
            }
            return Err(anyhow::anyhow!("Project '{}' not found in list", self.config.project_number));
        }

        let project_element = project_element.unwrap();

        // Select project (click on it) - make sure we click exactly on the project
        self.log("Choosing project...".to_string(), LogLevel::Info).await;

        // Try to scroll to project element if still valid
        if let Err(_) = self.browser.execute_script("arguments[0].scrollIntoView(true);", vec![project_element.clone()]).await {
            self.log("Couldn't scroll to element, continuing".to_string(), LogLevel::Debug).await;
        }

        // Click on the project element
        match project_element.click().await {
            Ok(_) => {
                self.log("Project clicked".to_string(), LogLevel::Debug).await;
            }
            Err(_) => {
                self.log("Direct click failed, trying alternative".to_string(), LogLevel::Debug).await;
                // Try to find the parent row and click on it instead
                if let Ok(parent_row) = project_element.find(thirtyfour::By::XPath("./ancestor-or-self::tr")).await {
                    parent_row.click().await.map_err(|_| anyhow::anyhow!("Could not click on project row"))?;
                    self.log("Clicked on parent row instead".to_string(), LogLevel::Debug).await;
                }
            }
        }

        // Look for 'Open' button
        self.log("Looking for 'Open' button...".to_string(), LogLevel::Info).await;
        let all_buttons = self.browser.find_elements(thirtyfour::By::Tag("button")).await?;
        self.log(format!("Found buttons after project click: {}", all_buttons.len()), LogLevel::Debug).await;

        let mut open_button = None;

        for (idx, btn) in all_buttons.iter().enumerate() {
            if let (Ok(btn_text), Ok(btn_value)) = (btn.text().await, btn.attr("value").await) {
                let text = btn_text.trim();
                let value = btn_value.unwrap_or_default();

                if !text.is_empty() || !value.is_empty() {
                    self.log(format!("Button {}: Text='{}' | Value='{}'", idx, text, value), LogLevel::Debug).await;
                }

                if text.to_lowercase().contains("öffnen") || text.to_lowercase().contains("open") {
                    if btn.is_displayed().await.unwrap_or(false) && btn.is_enabled().await.unwrap_or(false) {
                        open_button = Some(btn.clone());
                        self.log(format!("'Open' button found: '{}'", text), LogLevel::Success).await;
                        break;
                    }
                }
            }
        }

        if let Some(open_button) = open_button {
            self.log("Clicking on 'Open' button...".to_string(), LogLevel::Info).await;
            open_button.click().await.map_err(|_| anyhow::anyhow!("Unable to click on 'Open' button"))?;
            self.log("'Open' button clicked".to_string(), LogLevel::Success).await;

            self.log("Waiting for fully loading the project...".to_string(), LogLevel::Info).await;
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            // Wait for sidebar using WebDriverWait equivalent
            // For now, just check if sidebar exists
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            if let Ok(_sidebar) = self.browser.find_element(thirtyfour::By::XPath("//div[contains(@class, 'tree') or contains(@class, 'sidebar')]")).await {
                self.log("Project sidebar found".to_string(), LogLevel::Success).await;
            } else {
                self.log("Project sidebar not found, still continuing".to_string(), LogLevel::Warning).await;
            }

            // Check if project was successfully opened
            let current_url = self.browser.get_current_url().await?;
            if current_url.contains(&self.config.project_number) ||
               current_url.to_lowercase().contains("project") ||
               current_url.to_lowercase().contains("viewer") ||
               current_url.to_lowercase().contains("view") {
                self.log(format!("Project '{}' successfully opened!", self.config.project_number), LogLevel::Success).await;
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                Ok(())
            } else if current_url != self.config.base_url {
                self.log("Navigated to new page, project probably opened".to_string(), LogLevel::Success).await;
                Ok(())
            } else {
                self.log("Project state unclear, still proceeding...".to_string(), LogLevel::Warning).await;
                Ok(())
            }
        } else {
            Err(anyhow::anyhow!("'Open' button not found"))
        }
    }

    async fn switch_to_list_view(&mut self) -> Result<()> {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Click on button with three dots
        self.log("Looking for buttons that are 'eplan-icon-button'".to_string(), LogLevel::Info).await;

        let buttons = self.browser.find_elements(thirtyfour::By::Tag("eplan-icon-button")).await?;
        self.log(format!("Found {} eplan-icon-button elements", buttons.len()), LogLevel::Info).await;

        for (i, btn) in buttons.iter().enumerate() {
            if !btn.is_displayed().await.unwrap_or(false) {
                continue;
            }

            // Check for the specific data-t attribute
            if let Ok(data_t) = btn.attr("data-t").await {
                if let Some(data_t_value) = data_t {
                    if !data_t_value.contains("ev-btn-page-more") {
                        continue;
                    }

                    // Check if popup is already open
                    if let Ok(class_attr) = btn.attr("class").await {
                        if let Some(class_value) = class_attr {
                            if class_value.contains("fl-pop-up-open") {
                                self.log("Three dots pop-up is already open".to_string(), LogLevel::Info).await;
                                break;
                            }
                        }
                    }

                    // Try to click the button
                    match btn.click().await {
                        Ok(_) => {
                            self.log("Clicked button with three dots.".to_string(), LogLevel::Info).await;
                            break;
                        }
                        Err(_) => {
                            return Err(anyhow::anyhow!("Can't click on button with three dots"));
                        }
                    }
                } else {
                    self.log(format!("Can't find button with three dots, called at index {}", i), LogLevel::Error).await;
                    continue;
                }
            } else {
                self.log(format!("No data-t attribute found for button {}", i), LogLevel::Debug).await;
                continue;
            }
        }

        // Now find the list view button in the dropdown
        let dropdown_buttons = self.browser.find_elements(thirtyfour::By::Tag("eplan-dropdown-item")).await?;

        for btn in dropdown_buttons {
            if !btn.is_displayed().await.unwrap_or(false) {
                continue;
            }

            // Check for the specific data-name attribute
            if let Ok(data_name) = btn.attr("data-name").await {
                if let Some(data_name_value) = data_name {
                    if !data_name_value.contains("ev-page-list-view-btn") {
                        continue;
                    }

                    match btn.click().await {
                        Ok(_) => {
                            self.log("Clicked 'List' Button".to_string(), LogLevel::Info).await;
                            return Ok(());
                        }
                        Err(_) => {
                            return Err(anyhow::anyhow!("Can't click on 'List' button"));
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Failed to switch to list view"))
    }

    async fn extract_tables(&mut self) -> Result<bool> {
        self.log("🚀 Starting systematic SPS table extraction...".to_string(), LogLevel::Info).await;

        // Initialize the table to store results
        let mut table = PlcTable::new(self.config.project_number.clone());

        // Find the scroll container
        self.log("🔍 Looking for scroll container 'cdk-virtual-scroll-viewport'...".to_string(), LogLevel::Debug).await;
        let scroll_container = match self.browser.find_element(thirtyfour::By::Css("cdk-virtual-scroll-viewport")).await {
            Ok(container) => {
                self.log("✅ Found scroll container successfully".to_string(), LogLevel::Success).await;
                container
            }
            Err(e) => {
                self.log(format!("❌ Could not find scroll container: {}", e), LogLevel::Error).await;
                return Err(anyhow::anyhow!("Scroll container not found: {}", e));
            }
        };

        // STEP 1: Scroll to the very top first (as user suggested)
        self.log("📍 STEP 1: Scrolling to top of container...".to_string(), LogLevel::Info).await;
        match self.browser.execute_script("arguments[0].scrollTop = 0", vec![scroll_container.clone()]).await {
            Ok(_) => {
                self.log("✅ Successfully scrolled to top (scrollTop = 0)".to_string(), LogLevel::Success).await;
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await; // Wait for content to load
            }
            Err(e) => {
                self.log(format!("⚠️ Could not scroll to top: {}", e), LogLevel::Warning).await;
            }
        }

        // STEP 2: Start systematic page-by-page processing
        self.log("📍 STEP 2: Starting systematic page-by-page processing...".to_string(), LogLevel::Info).await;

        let mut last_height = -1i64;
        let mut plc_diagram_pages = std::collections::HashSet::new();
        let mut extracted_page_texts = Vec::new();
        let mut total_pages_processed = 0;
        let mut scroll_iteration = 0;

        // Main scrolling loop
        loop {
            scroll_iteration += 1;
            self.log(format!("🔄 SCROLL ITERATION #{}: Scanning for page items...", scroll_iteration), LogLevel::Info).await;

            // Find visible items
            let visible_items = match self.browser.find_elements(thirtyfour::By::Tag("pv-page-list-item")).await {
                Ok(items) => {
                    self.log(format!("📋 Found {} visible page items in iteration #{}", items.len(), scroll_iteration), LogLevel::Debug).await;
                    items
                }
                Err(e) => {
                    self.log(format!("⚠️ Could not find page list items: {}", e), LogLevel::Warning).await;
                    break;
                }
            };

            // Process each visible item systematically
            for i in 0..visible_items.len() {
                total_pages_processed += 1;

                // Re-fetch element to avoid stale references
                if let Ok(current_items) = self.browser.find_elements(thirtyfour::By::Tag("pv-page-list-item")).await {
                    if i >= current_items.len() {
                        self.log(format!("⚠️ Item index {} out of bounds ({}), skipping", i, current_items.len()), LogLevel::Warning).await;
                        continue;
                    }

                    let item = &current_items[i];
                    self.log(format!("🔍 Processing page item #{} (iteration #{}, item #{})", total_pages_processed, scroll_iteration, i+1), LogLevel::Debug).await;

                    // Check for PLC-Diagram using the correct selectors from screenshots
                    let mut is_plc_diagram = false;
                    let mut found_text = String::new();

                    // Method 1: Look for .ev-description.ev-hi elements (from screenshot analysis)
                    if let Ok(description_elements) = item.find_all(thirtyfour::By::Css(".ev-description.ev-hi")).await {
                        self.log(format!("🔍 Found {} .ev-description.ev-hi elements", description_elements.len()), LogLevel::Debug).await;

                        for desc_element in &description_elements {
                            if let Ok(text) = desc_element.text().await {
                                self.log(format!("📝 .ev-description.ev-hi text: '{}'", text), LogLevel::Debug).await;
                                if text.contains("PLC-Diagram") {
                                    is_plc_diagram = true;
                                    found_text = text.clone();
                                    self.log(format!("✅ FOUND PLC-Diagram in .ev-description.ev-hi: '{}'", text), LogLevel::Success).await;
                                    break;
                                }
                            }
                        }
                    }

                    // Method 2: Fallback - look in all nested elements
                    if !is_plc_diagram {
                        if let Ok(all_nested) = item.find_all(thirtyfour::By::XPath(".//*[contains(text(), 'PLC-Diagram')]")).await {
                            if !all_nested.is_empty() {
                                if let Ok(text) = all_nested[0].text().await {
                                    is_plc_diagram = true;
                                    found_text = text.clone();
                                    self.log(format!("✅ FOUND PLC-Diagram via XPath fallback: '{}'", text), LogLevel::Success).await;
                                }
                            }
                        }
                    }

                    // Method 3: Ultimate fallback - check all text content
                    if !is_plc_diagram {
                        if let Ok(item_text) = item.text().await {
                            self.log(format!("📝 Full item text: '{}'", item_text.replace("\n", " ").trim()), LogLevel::Debug).await;
                            if item_text.contains("PLC-Diagram") {
                                is_plc_diagram = true;
                                found_text = item_text.clone();
                                self.log(format!("✅ FOUND PLC-Diagram in full text: '{}'", item_text.replace("\n", " ").trim()), LogLevel::Success).await;
                            }
                        }
                    }

                    if is_plc_diagram {
                        // Get unique identifier using outerHTML
                        if let Ok(Some(outer_html)) = item.attr("outerHTML").await {
                            if plc_diagram_pages.insert(outer_html) {
                                self.log(format!("🎯 CLICKING PLC-Diagram page #{} (found text: '{}')", plc_diagram_pages.len(), found_text.replace("\n", " ").trim()), LogLevel::Info).await;

                                // Small delay to stabilize
                                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                                // Click the item
                                match item.click().await {
                                    Ok(_) => {
                                        self.log(format!("✅ Successfully clicked PLC page #{}", plc_diagram_pages.len()), LogLevel::Success).await;

                                        // Wait for page to update
                                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                                        // Extract content from this page
                                        self.log(format!("⚙️ Extracting content from PLC page #{}...", plc_diagram_pages.len()), LogLevel::Info).await;
                                        match self.extract_current_plc_diagram_page().await {
                                            Ok(extracted_text) => {
                                                if !extracted_text.is_empty() {
                                                    extracted_page_texts.push(extracted_text);
                                                    self.log(format!("✅ Successfully extracted content from PLC page #{} (total: {})", plc_diagram_pages.len(), extracted_page_texts.len()), LogLevel::Success).await;
                                                } else {
                                                    self.log(format!("⚠️ No content extracted from PLC page #{}", plc_diagram_pages.len()), LogLevel::Warning).await;
                                                }
                                            }
                                            Err(e) => {
                                                self.log(format!("❌ Error extracting content from PLC page #{}: {}", plc_diagram_pages.len(), e), LogLevel::Error).await;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        self.log(format!("❌ Failed to click PLC page #{}: {}", plc_diagram_pages.len(), e), LogLevel::Error).await;
                                    }
                                }
                            } else {
                                self.log(format!("⚠️ PLC page already processed (duplicate): '{}'", found_text.replace("\n", " ").trim()), LogLevel::Debug).await;
                            }
                        }
                    } else {
                        self.log(format!("⚪ Page item #{} is not a PLC-Diagram (skipped)", total_pages_processed), LogLevel::Debug).await;
                    }
                }

                // Small delay between items to avoid overwhelming the browser
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            // Scroll down for next batch of items
            self.log(format!("⬇️ Scrolling down for next batch (iteration #{})...", scroll_iteration), LogLevel::Debug).await;
            if let Err(e) = self.browser.execute_script("arguments[0].scrollTop += 400", vec![scroll_container.clone()]).await {
                self.log(format!("❌ Could not scroll down: {}", e), LogLevel::Warning).await;
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            // Check if reached bottom
            if let Ok(new_height) = self.browser.execute_script_and_get_value("return arguments[0].scrollTop", vec![scroll_container.clone()]).await {
                if let Some(height_num) = new_height.as_i64() {
                    self.log(format!("📏 Current scroll position: {} (previous: {})", height_num, last_height), LogLevel::Debug).await;

                    if height_num == last_height {
                        self.log("🏁 Reached bottom of scroll container - extraction complete!".to_string(), LogLevel::Info).await;
                        break; // reached bottom
                    }
                    last_height = height_num;
                } else {
                    self.log("⚠️ Could not get scroll height, assuming bottom reached".to_string(), LogLevel::Warning).await;
                    break;
                }
            } else {
                self.log("❌ Could not execute scroll height script, stopping".to_string(), LogLevel::Error).await;
                break;
            }
        }

        // Final results summary
        self.log("📊 EXTRACTION SUMMARY:".to_string(), LogLevel::Info).await;
        self.log(format!("   📋 Total pages scanned: {}", total_pages_processed), LogLevel::Info).await;
        self.log(format!("   🎯 PLC-Diagram pages found: {}", plc_diagram_pages.len()), LogLevel::Info).await;
        self.log(format!("   📄 Pages with extracted content: {}", extracted_page_texts.len()), LogLevel::Info).await;
        self.log(format!("   🔄 Scroll iterations: {}", scroll_iteration), LogLevel::Info).await;

        if !extracted_page_texts.is_empty() {
            // Save extracted content to JSON file for debugging
            if let Err(e) = self.save_extracted_pages_to_json(&extracted_page_texts).await {
                self.log(format!("⚠️ Failed to save extracted_pages.json: {}", e), LogLevel::Warning).await;
            } else {
                self.log("✅ Results saved to extracted_pages.json for debugging".to_string(), LogLevel::Success).await;
            }

            // Parse and add entries to table
            self.log("⚙️ Parsing extracted content and building table...".to_string(), LogLevel::Info).await;
            for (i, page_text) in extracted_page_texts.iter().enumerate() {
                self.log(format!("⚙️ Parsing page {} of {}...", i+1, extracted_page_texts.len()), LogLevel::Debug).await;
                self.parse_and_add_to_table(page_text, &mut table).await;
            }

            self.log(format!("✅ Final table contains {} entries", table.entries.len()), LogLevel::Success).await;
        } else {
            self.log("⚠️ No content was extracted from any pages".to_string(), LogLevel::Warning).await;
        }

        // Store the table and return success status
        self.extracted_table = Some(table);
        Ok(!plc_diagram_pages.is_empty())
    }

    async fn wait_for_svg_content(&self) -> Result<()> {
        // Try to wait for SVG content to load (similar to Python WebDriverWait)
        for _ in 0..10 { // 5 second timeout
            if let Ok(_) = self.browser.find_element(thirtyfour::By::Tag("svg")).await {
                return Ok(());
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        Err(anyhow::anyhow!("SVG content not found"))
    }

    async fn extract_current_plc_diagram_page(&self) -> Result<String> {
        // This method should match Python extract_current_plc_diagram_page_advanced()
        let mut extracted_content = Vec::new();

        // Try to extract content (Python line 1032-1056)
        match self.browser.get_page_source().await {
            Ok(page_source) => {
                // Use regex patterns exactly like Python (line 1038-1042)
                let text_pattern = regex::Regex::new(r"<text[^>]*>([^<]+)</text>").unwrap();
                let tspan_pattern = regex::Regex::new(r"<tspan[^>]*>([^<]+)</tspan>").unwrap();

                // Find text matches (Python line 1039)
                for capture in text_pattern.captures_iter(&page_source) {
                    if let Some(text_match) = capture.get(1) {
                        extracted_content.push(text_match.as_str().to_string());
                    }
                }

                // Extend with tspan matches (Python line 1041-1042)
                for capture in tspan_pattern.captures_iter(&page_source) {
                    if let Some(text_match) = capture.get(1) {
                        extracted_content.push(text_match.as_str().to_string());
                    }
                }

                if !extracted_content.is_empty() {
                    self.log(format!("Regex found {} text matches", extracted_content.len()), LogLevel::Debug).await;

                    // Filter content (Python line 1047-1053)
                    let mut filtered_content = Vec::new();
                    for text in extracted_content {
                        let trimmed = text.trim();
                        if !trimmed.is_empty() && trimmed.len() > 2 {
                            // Filter out unwanted elements (Python line 1050-1052)
                            if !["Date", "Datum", "ET 200SP"].iter().any(|skip| trimmed.contains(skip)) {
                                filtered_content.push(trimmed.to_string());
                            }
                        }
                    }
                    extracted_content = filtered_content;
                }
            }
            Err(e) => {
                self.log(format!("Page source extraction failed: {}", e), LogLevel::Error).await;
                return Ok(String::new());
            }
        }

        if !extracted_content.is_empty() {
            // Remove duplicates while preserving order (Python line 1058-1064)
            let mut seen = std::collections::HashSet::new();
            let mut unique_content = Vec::new();

            for item in extracted_content {
                if !seen.contains(&item) {
                    seen.insert(item.clone());
                    unique_content.push(item);
                }
            }

            let result = unique_content.join(" ");
            self.log(format!("Successfully extracted {} unique text elements", unique_content.len()), LogLevel::Success).await;

            // Parse the data (Python line 1071-1073)
            self.log("TRYING TO CALL PARSE".to_string(), LogLevel::Debug).await;
            let parsed_data = self.parse_plc_data(&result);

            // Format result like Python (line 1073: "; ".join(" ".join(d.values()) for d in parsed_data))
            let result_string = parsed_data.into_iter()
                .map(|entry| format!("{} {}", entry.address, entry.symbol_name))
                .collect::<Vec<_>>()
                .join("; ");

            Ok(result_string)
        } else {
            self.log("No content could be extracted with any method".to_string(), LogLevel::Error).await;

            // Debug: Save page source for manual analysis (Python line 1079-1087)
            if let Ok(page_source) = self.browser.get_page_source().await {
                let debug_file = format!("debug_page_source_{}.html", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
                if std::fs::write(&debug_file, &page_source).is_ok() {
                    self.log(format!("Saved page source for debugging: {}", debug_file), LogLevel::Debug).await;
                }
            }

            Ok(String::new())
        }
    }

    async fn save_extracted_pages_to_json(&self, pages: &[String]) -> Result<()> {
        let json_content = serde_json::to_string_pretty(pages)?;
        std::fs::write("extracted_pages.json", json_content)?;
        Ok(())
    }

    async fn parse_and_add_to_table(&self, page_text: &str, table: &mut PlcTable) {
        let entries = self.parse_plc_data(page_text);
        for entry in entries {
            table.entries.push(entry);
        }
    }

    fn parse_plc_data(&self, input_string: &str) -> Vec<PlcEntry> {
        let mut results = Vec::new();

        // Split string into lines
        let normalized = input_string.replace("\r\n", "\n").replace('\r', "\n");
        let lines: Vec<&str> = normalized.split('\n').collect();

        // Regex patterns from Python
        let address_pattern = regex::Regex::new(r"\b([IQ]W?\d+\.\d+|[IQ]W\d+)\b").unwrap();
        let function_pattern = regex::Regex::new(r"([A-Za-z][A-Za-z\s]+(?:\d+\.)+\d+(?:\s+[A-Z]+)?)").unwrap();

        let mut current_function = String::new();

        for line in lines {
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            if let Some(address_match) = address_pattern.find(line) {
                let address = address_match.as_str().to_string();
                let text_before_address = &line[..address_match.start()].trim();

                if let Some(function_match) = function_pattern.find(text_before_address) {
                    current_function = function_match.as_str().trim().to_string();
                } else if !text_before_address.is_empty() && !text_before_address.starts_with('=') {
                    let parts: Vec<&str> = text_before_address.split_whitespace().collect();
                    let valid_parts: Vec<&str> = parts.into_iter()
                        .filter(|p| !p.starts_with('=') && !p.starts_with(':'))
                        .collect();
                    if !valid_parts.is_empty() {
                        current_function = valid_parts.join(" ");
                    }
                }

                if !current_function.is_empty() {
                    results.push(PlcEntry {
                        address: address.clone(),
                        symbol_name: current_function.clone(),
                        data_type: crate::models::PlcDataType::from_address(&address),
                        page: "".to_string(), // Will be set elsewhere if needed
                        selected: false,
                        comment: String::new(),
                    });
                }
            }
        }

        results
    }

    pub async fn close(&self) -> Result<()> {
        // Close browser first
        self.browser.quit().await?;

        // Then stop ChromeDriver
        self.chromedriver_manager.stop_driver().await?;

        Ok(())
    }
}