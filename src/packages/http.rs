//! Shared HTTP client for package downloads
//!
//! Provides a common HTTP client with consistent error handling,
//! used by SSC, GitHub, and Net downloaders.

use crate::error::{Error, Result};
use reqwest::blocking::Client;
use std::time::Duration;

/// HTTP client timeout for all package operations
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Shared HTTP client for stacy package operations
pub struct StacyHttpClient {
    client: Client,
}

impl Default for StacyHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl StacyHttpClient {
    /// Create a new HTTP client with stacy's default settings
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(concat!("stacy/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    /// Get the underlying reqwest client for custom requests (e.g., JSON API calls)
    pub fn inner(&self) -> &Client {
        &self.client
    }

    /// Download a URL and return its content as text
    ///
    /// Returns appropriate errors for timeouts, connection failures,
    /// 404s, and other HTTP errors.
    pub fn download_text(&self, url: &str) -> Result<String> {
        let response = self.send_request(url)?;
        self.check_status(&response, url)?;

        response
            .text()
            .map_err(|e| Error::Network(format!("Failed to read response: {}", e)))
    }

    /// Download a URL and return its content as bytes
    ///
    /// Returns appropriate errors for timeouts, connection failures,
    /// 404s, and other HTTP errors.
    pub fn download_bytes(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.send_request(url)?;
        self.check_status(&response, url)?;

        response
            .bytes()
            .map(|b| b.to_vec())
            .map_err(|e| Error::Network(format!("Failed to read response: {}", e)))
    }

    /// Send a GET request with standardized error handling
    fn send_request(&self, url: &str) -> Result<reqwest::blocking::Response> {
        self.client.get(url).send().map_err(|e| {
            if e.is_timeout() {
                Error::Network(format!("Request timed out: {}", url))
            } else if e.is_connect() {
                Error::Network(format!("Connection failed: {}", url))
            } else {
                Error::Network(format!("HTTP error: {}", e))
            }
        })
    }

    /// Check HTTP response status and return appropriate errors
    fn check_status(&self, response: &reqwest::blocking::Response, url: &str) -> Result<()> {
        if !response.status().is_success() {
            let status = response.status();
            if status.as_u16() == 404 {
                return Err(Error::Config(format!("Not found: {}", url)));
            }
            return Err(Error::Network(format!(
                "HTTP {} for {}",
                status.as_u16(),
                url
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creates_successfully() {
        let client = StacyHttpClient::new();
        let _ = client.inner();
    }

    #[test]
    fn test_connection_refused_returns_error() {
        let client = StacyHttpClient::new();
        let result = client.download_text("http://127.0.0.1:1/nonexistent");
        assert!(result.is_err());
    }
}
