//! Test context and setup utilities for E2E tests

use super::proxy::CapiProxy;
use github_copilot::{Client, ClientOptions};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static CACHED_CLI_PATH: OnceLock<Mutex<Option<String>>> = OnceLock::new();

/// Get the CLI path, discovering it once and caching
pub fn cli_path() -> Option<String> {
    let cache = CACHED_CLI_PATH.get_or_init(|| {
        let mut path = None;
        
        // Check environment variable first
        if let Ok(p) = env::var("COPILOT_CLI_PATH") {
            if PathBuf::from(&p).exists() {
                path = Some(p);
            }
        }

        // Look for CLI in sibling nodejs directory's node_modules
        if path.is_none() {
            let possible_path = "../../nodejs/node_modules/@github/copilot/index.js";
            if let Ok(abs_path) = fs::canonicalize(possible_path) {
                if abs_path.exists() {
                    path = Some(abs_path.to_string_lossy().to_string());
                }
            }
        }
        
        Mutex::new(path)
    });

    cache.lock().unwrap().clone()
}

/// TestContext holds shared resources for E2E tests
pub struct TestContext {
    pub cli_path: String,
    pub home_dir: PathBuf,
    pub work_dir: PathBuf,
    pub proxy_url: String,
    proxy: CapiProxy,
}

impl TestContext {
    /// Create a new test context with isolated directories and a replaying proxy
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let cli_path = cli_path()
            .ok_or("CLI not found. Set COPILOT_CLI_PATH or run 'npm install' in nodejs/")?;

        if !PathBuf::from(&cli_path).exists() {
            return Err(format!("CLI not found at {}", cli_path).into());
        }

        let home_dir = tempfile::tempdir()?.into_path();
        let work_dir = tempfile::tempdir()?.into_path();

        let proxy = CapiProxy::new();
        let proxy_url = proxy.start()?;

        Ok(Self {
            cli_path,
            home_dir,
            work_dir,
            proxy_url,
            proxy,
        })
    }

    /// Configure the proxy for a specific test
    /// Call this at the start of each test
    pub fn configure_for_test(&self, test_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Format: test/snapshots/<testFile>/<testName>.yaml
        let sanitized = test_name.to_lowercase().replace("::", "_").replace(" ", "_");
        let parts: Vec<&str> = sanitized.split('_').collect();

        let test_file = parts.get(0).unwrap_or(&"unknown");
        let test_case = parts.get(1..).map(|s| s.join("_")).unwrap_or_default();

        let snapshot_path = format!("../../test/snapshots/{}/{}.yaml", test_file, test_case);
        let abs_snapshot_path = fs::canonicalize(&snapshot_path)
            .map_err(|e| format!("Snapshot not found at {}: {}", snapshot_path, e))?;

        self.proxy.configure(
            abs_snapshot_path.to_str().unwrap(),
            self.work_dir.to_str().unwrap(),
        )?;

        Ok(())
    }

    /// Get environment variables configured for isolated testing
    pub fn env(&self) -> Vec<(String, String)> {
        vec![
            ("COPILOT_API_URL".to_string(), self.proxy_url.clone()),
            ("XDG_CONFIG_HOME".to_string(), self.home_dir.to_string_lossy().to_string()),
            ("XDG_STATE_HOME".to_string(), self.home_dir.to_string_lossy().to_string()),
        ]
    }

    /// Create a Client configured for this test context
    pub fn new_client(&self) -> Client {
        let mut options = ClientOptions::new()
            .cli_path(self.cli_path.clone())
            .cwd(self.work_dir.to_string_lossy().to_string());

        // Set environment variables
        options.env = Some(self.env());

        Client::new(options)
    }

    /// Get captured HTTP exchanges from the proxy
    #[allow(dead_code)]
    pub fn get_exchanges(&self) -> Result<Vec<super::proxy::ParsedHttpExchange>, Box<dyn std::error::Error>> {
        self.proxy.get_exchanges()
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        let _ = self.proxy.stop();
        let _ = fs::remove_dir_all(&self.home_dir);
        let _ = fs::remove_dir_all(&self.work_dir);
    }
}
