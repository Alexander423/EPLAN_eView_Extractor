use anyhow::Result;
use crate::scraper::browser::BrowserDriver;
use tokio::time::{sleep, Duration};
use thirtyfour::prelude::*;

pub struct MicrosoftAuth;

impl MicrosoftAuth {
    pub async fn login(
        browser: &BrowserDriver,
        username: &str,
        password: &str,
    ) -> Result<bool> {
        // Wait for Microsoft login page
        sleep(Duration::from_secs(2)).await;

        // Find and fill email field
        if let Some(email_field) = browser.find_email_field().await? {
            browser.send_keys(&email_field, username).await?;

            // Click next button
            if let Some(next_btn) = browser.find_submit_button().await? {
                browser.click_element(&next_btn).await?;
            } else {
                // Press Enter if no button found
                email_field.send_keys(Key::Return).await?;
            }

            sleep(Duration::from_secs(3)).await;

            // Find and fill password field
            if let Some(password_field) = browser.find_password_field().await? {
                browser.send_keys(&password_field, password).await?;

                // Click sign in button
                if let Some(signin_btn) = browser.find_submit_button().await? {
                    browser.click_element(&signin_btn).await?;
                } else {
                    password_field.send_keys(Key::Return).await?;
                }

                sleep(Duration::from_secs(2)).await;

                // Handle "Stay signed in?" prompt if it appears
                Self::handle_stay_signed_in(browser).await?;

                // Wait for redirect back to EPLAN
                sleep(Duration::from_secs(5)).await;

                // Check if login was successful
                let current_url = browser.get_current_url().await?;
                if !current_url.contains("login") &&
                   (current_url.contains("eview") || current_url.contains("eplan")) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    async fn handle_stay_signed_in(browser: &BrowserDriver) -> Result<()> {
        // Try to find and click "Yes" button for stay signed in
        let yes_selectors = vec![
            By::Css("input[id='idSIButton9']"),
            By::Css("input[value='Yes']"),
            By::Css("input[value='Ja']"),
            By::Css("button[id='idSIButton9']"),
        ];

        for _ in 0..10 {
            for selector in &yes_selectors {
                if let Ok(element) = browser.find_element(selector.clone()).await {
                    if element.is_displayed().await.unwrap_or(false)
                        && element.is_enabled().await.unwrap_or(false) {
                        browser.click_element(&element).await?;
                        return Ok(());
                    }
                }
            }
            sleep(Duration::from_millis(500)).await;
        }

        Ok(())
    }

    pub async fn click_microsoft_button(browser: &BrowserDriver) -> Result<bool> {
        // Try multiple times to find the Microsoft login button
        for _attempt in 0..15 {
            if let Some(button) = browser.find_microsoft_button().await? {
                browser.click_element(&button).await?;
                sleep(Duration::from_secs(1)).await;

                // Check if we navigated to Microsoft login
                let url = browser.get_current_url().await?;
                if url.contains("login.microsoft") {
                    return Ok(true);
                }
            }

            sleep(Duration::from_secs(1)).await;
        }

        Err(anyhow::anyhow!("Could not find Microsoft login button"))
    }
}