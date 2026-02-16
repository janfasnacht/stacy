/// Stata binary detection with precedence chain
///
/// # Precedence Order
///
/// 1. CLI flag `--engine` (highest priority)
/// 2. Environment variable `$STATA_BINARY` (machine-specific)
/// 3. User config `~/.config/stacy/config.toml` (stata_binary field)
/// 4. Auto-detection via PATH search (lowest priority)
///
/// Note: Stata binary is NOT in project config (stacy.toml) because it's
/// machine-specific and shouldn't be committed to version control.
///
/// # Auto-Detection Strategy
///
/// Search PATH for Stata binaries in preference order:
/// - `stata-mp` (Multi-core, preferred)
/// - `stata-se` (Standard Edition)
/// - `stata-be` (Basic Edition)
/// - `stata` (Generic fallback)
///
/// # Platform-Specific Locations
///
/// **macOS:**
/// - `/Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp`
/// - `/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp`
/// - PATH search
///
/// **Linux:**
/// - `/usr/local/stata18/stata-mp`
/// - `/usr/local/stata17/stata-mp`
/// - `/usr/local/stata/stata-mp`
/// - PATH search
///
/// **Windows:**
/// - `C:\Program Files\StataNow\StataMP-64.exe` (and SE, IC, BE)
/// - `C:\Program Files\Stata18\StataMP-64.exe` (and SE, IC, BE)
/// - `C:\Program Files\Stata17\StataMP-64.exe` (and SE, IC, BE)
/// - `C:\Program Files\Stata16\StataMP-64.exe` (and SE, IC)
/// - `C:\Program Files\Stata\StataMP-64.exe` (version-agnostic)
/// - `C:\Program Files (x86)\Stata*\StataSE.exe` (32-bit installations)
/// - PATH search
use crate::error::{Error, Result};
use std::env;
use std::path::Path;
use std::process::Command;

/// Stata binary preference order for auto-detection
const BINARY_NAMES: &[&str] = &["stata-mp", "stata-se", "stata-be", "stata"];

/// macOS-specific Stata application locations
#[cfg(target_os = "macos")]
const MACOS_APP_LOCATIONS: &[&str] = &[
    "/Applications/StataNow/StataMP.app/Contents/MacOS/stata-mp",
    "/Applications/StataNow/StataSE.app/Contents/MacOS/stata-se",
    "/Applications/Stata/StataMP.app/Contents/MacOS/stata-mp",
    "/Applications/Stata/StataSE.app/Contents/MacOS/stata-se",
];

/// Linux-specific Stata installation locations
#[cfg(target_os = "linux")]
const LINUX_LOCATIONS: &[&str] = &[
    "/usr/local/stata18/stata-mp",
    "/usr/local/stata17/stata-mp",
    "/usr/local/stata/stata-mp",
];

/// Windows-specific Stata installation locations
#[cfg(target_os = "windows")]
const WINDOWS_LOCATIONS: &[&str] = &[
    // StataNow (newest, 64-bit)
    r"C:\Program Files\StataNow\StataMP-64.exe",
    r"C:\Program Files\StataNow\StataSE-64.exe",
    r"C:\Program Files\StataNow\StataIC-64.exe",
    r"C:\Program Files\StataNow\StataBE-64.exe",
    // Stata 18 (64-bit)
    r"C:\Program Files\Stata18\StataMP-64.exe",
    r"C:\Program Files\Stata18\StataSE-64.exe",
    r"C:\Program Files\Stata18\StataIC-64.exe",
    r"C:\Program Files\Stata18\StataBE-64.exe",
    // Stata 17 (64-bit)
    r"C:\Program Files\Stata17\StataMP-64.exe",
    r"C:\Program Files\Stata17\StataSE-64.exe",
    r"C:\Program Files\Stata17\StataIC-64.exe",
    r"C:\Program Files\Stata17\StataBE-64.exe",
    // Stata 16 (64-bit)
    r"C:\Program Files\Stata16\StataMP-64.exe",
    r"C:\Program Files\Stata16\StataSE-64.exe",
    r"C:\Program Files\Stata16\StataIC-64.exe",
    // Generic version-agnostic (64-bit)
    r"C:\Program Files\Stata\StataMP-64.exe",
    r"C:\Program Files\Stata\StataSE-64.exe",
    r"C:\Program Files\Stata\StataIC-64.exe",
    // Program Files (x86) for 32-bit installations
    r"C:\Program Files (x86)\Stata18\StataSE.exe",
    r"C:\Program Files (x86)\Stata18\StataIC.exe",
    r"C:\Program Files (x86)\Stata17\StataSE.exe",
    r"C:\Program Files (x86)\Stata17\StataIC.exe",
    r"C:\Program Files (x86)\Stata16\StataSE.exe",
    r"C:\Program Files (x86)\Stata16\StataIC.exe",
    r"C:\Program Files (x86)\Stata\StataSE.exe",
    r"C:\Program Files (x86)\Stata\StataIC.exe",
];

/// Detect Stata binary using precedence chain
///
/// # Arguments
///
/// * `cli_engine` - Optional binary from CLI `--engine` flag
///
/// # Returns
///
/// Path to Stata binary (absolute if from auto-detection)
///
/// # Errors
///
/// Returns error if no Stata binary found in any location
pub fn detect_stata_binary(cli_engine: Option<&str>) -> Result<String> {
    // 1. CLI flag (highest priority)
    if let Some(binary) = cli_engine {
        if verify_binary(binary)? {
            return Ok(binary.to_string());
        } else {
            return Err(Error::Execution(format!(
                "Stata binary specified via --engine not found or not executable: {}",
                binary
            )));
        }
    }

    // 2. Environment variable ($STATA_BINARY)
    if let Ok(binary) = env::var("STATA_BINARY") {
        if verify_binary(&binary)? {
            return Ok(binary);
        } else {
            return Err(Error::Execution(format!(
                "Stata binary from $STATA_BINARY not found or not executable: {}",
                binary
            )));
        }
    }

    // 3. User config (~/.config/stacy/config.toml)
    if let Some(binary) = get_user_config_binary()? {
        if verify_binary(&binary)? {
            return Ok(binary);
        } else {
            return Err(Error::Execution(format!(
                "Stata binary from ~/.config/stacy/config.toml not found or not executable: {}",
                binary
            )));
        }
    }

    // 4. Auto-detection
    auto_detect_binary()
}

/// Get stata_binary from user config if available
fn get_user_config_binary() -> Result<Option<String>> {
    use crate::project::user_config::load_user_config;

    match load_user_config() {
        Ok(Some(config)) => Ok(config.stata_binary),
        Ok(None) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Auto-detect Stata binary by searching standard locations and PATH
fn auto_detect_binary() -> Result<String> {
    // Try platform-specific locations first
    if let Some(binary) = try_platform_locations() {
        return Ok(binary);
    }

    // Try PATH search
    if let Some(binary) = try_path_search() {
        return Ok(binary);
    }

    // No Stata binary found
    Err(Error::Execution(
        "Stata binary not found. Tried:\n\
         - Platform-specific locations\n\
         - PATH search for: stata-mp, stata-se, stata-be, stata\n\n\
         Fix: Install Stata, or set STATA_ENGINE environment variable, or use --engine flag"
            .to_string(),
    ))
}

/// Try platform-specific Stata installation locations
fn try_platform_locations() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        for location in MACOS_APP_LOCATIONS {
            if Path::new(location).is_file() && is_executable(location) {
                return Some(location.to_string());
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        for location in LINUX_LOCATIONS {
            if Path::new(location).is_file() && is_executable(location) {
                return Some(location.to_string());
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        for location in WINDOWS_LOCATIONS {
            if Path::new(location).is_file() && is_executable(location) {
                return Some(location.to_string());
            }
        }
    }

    None
}

/// Search PATH for Stata binaries
fn try_path_search() -> Option<String> {
    for binary_name in BINARY_NAMES {
        if let Some(path) = find_in_path(binary_name) {
            return Some(path);
        }
    }
    None
}

/// Find binary in PATH
fn find_in_path(binary_name: &str) -> Option<String> {
    // Use `which` command on Unix, `where` on Windows
    #[cfg(not(target_os = "windows"))]
    let which_cmd = "which";
    #[cfg(target_os = "windows")]
    let which_cmd = "where";

    if let Ok(output) = Command::new(which_cmd).arg(binary_name).output() {
        if output.status.success() {
            if let Ok(path) = String::from_utf8(output.stdout) {
                let path = path.trim();
                if !path.is_empty() && Path::new(path).is_file() {
                    return Some(path.to_string());
                }
            }
        }
    }

    None
}

/// Verify that a binary exists and is executable
fn verify_binary(binary: &str) -> Result<bool> {
    let path = Path::new(binary);

    // Check if path exists
    if !path.exists() {
        return Ok(false);
    }

    // Check if it's a file (not a directory)
    if !path.is_file() {
        return Ok(false);
    }

    // Check if executable
    Ok(is_executable(binary))
}

/// Check if a file is executable
#[cfg(unix)]
fn is_executable(path: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(path) {
        let permissions = metadata.permissions();
        // Check if executable bit is set (0o111 = --x--x--x)
        permissions.mode() & 0o111 != 0
    } else {
        false
    }
}

#[cfg(not(unix))]
fn is_executable(_path: &str) -> bool {
    // On Windows, all .exe files are executable
    // For now, just return true if file exists
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_names_order() {
        // Verify preference order
        assert_eq!(BINARY_NAMES[0], "stata-mp");
        assert_eq!(BINARY_NAMES[1], "stata-se");
        assert_eq!(BINARY_NAMES[2], "stata-be");
        assert_eq!(BINARY_NAMES[3], "stata");
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_locations_defined() {
        assert!(!MACOS_APP_LOCATIONS.is_empty());
        // Should include StataNow path
        assert!(MACOS_APP_LOCATIONS.iter().any(|p| p.contains("StataNow")));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_locations_defined() {
        assert!(!LINUX_LOCATIONS.is_empty());
        // Should include stata18 path
        assert!(LINUX_LOCATIONS.iter().any(|p| p.contains("stata18")));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_locations_defined() {
        assert!(!WINDOWS_LOCATIONS.is_empty());
        // Should include StataNow path
        assert!(WINDOWS_LOCATIONS.iter().any(|p| p.contains("StataNow")));
        // Should include both 64-bit and 32-bit locations
        assert!(WINDOWS_LOCATIONS.iter().any(|p| p.contains("-64.exe")));
        assert!(WINDOWS_LOCATIONS
            .iter()
            .any(|p| p.contains("Program Files (x86)")));
        // Should include multiple versions
        assert!(WINDOWS_LOCATIONS.iter().any(|p| p.contains("Stata18")));
        assert!(WINDOWS_LOCATIONS.iter().any(|p| p.contains("Stata17")));
    }

    #[test]
    fn test_verify_nonexistent_binary() {
        let result = verify_binary("/nonexistent/stata-mp");
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn test_cli_precedence() {
        // CLI flag should override everything
        // This test uses a fake binary path to verify precedence logic
        let result = detect_stata_binary(Some("/fake/stata-mp"));
        assert!(result.is_err()); // Fails because binary doesn't exist
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("--engine not found"));
    }
}
