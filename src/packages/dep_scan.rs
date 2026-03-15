//! Post-install dependency scanning for Stata packages
//!
//! Scans a package's `.ado` files for dependency patterns (`require`, `which`,
//! `findfile`) and reports any that are missing from the project's config.
//! This catches implicit dependencies that would otherwise only surface at
//! runtime when Stata errors out.

use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

/// Matches: `require ftools`, `cap require reghdfe`, `capture require pkg`
static REQUIRE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?im)^\s*(?:cap(?:ture)?\s+)?require\s+(\w+)").unwrap());

/// Matches: `which ftools`, `cap which reghdfe`, `capture which pkg.ado`
static WHICH_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?im)^\s*(?:cap(?:ture)?\s+)?which\s+(\w+)(?:\.ado)?"#).unwrap()
});

/// Matches: `findfile "moremata.ado"`, `findfile "pkg.mata"`, `findfile "pkg.hlp"`
static FINDFILE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?im)^\s*(?:cap(?:ture)?\s+)?findfile\s+"(\w+)\.\w+""#).unwrap()
});

/// Stata built-in commands and keywords that should never be flagged as deps.
const BUILTINS: &[&str] = &[
    "using",
    "stata",
    "merge",
    "sort",
    "by",
    "egen",
    "generate",
    "replace",
    "drop",
    "keep",
    "rename",
    "reshape",
    "collapse",
    "append",
    "save",
    "use",
    "describe",
    "summarize",
    "tabulate",
    "regress",
    "logit",
    "probit",
    "mata",
    "program",
    "capture",
    "quietly",
    "noisily",
    "display",
    "set",
    "local",
    "global",
    "tempvar",
    "tempname",
    "tempfile",
    "foreach",
    "forvalues",
    "while",
    "if",
    "else",
    "preserve",
    "restore",
    "assert",
    "confirm",
    "matrix",
    "scalar",
    "return",
    "ereturn",
    "sreturn",
    "estimates",
    "constraint",
    "predict",
    "margins",
    "test",
    "lincom",
    "nlcom",
    "bootstrap",
    "jackknife",
    "simulate",
    "graph",
    "label",
    "notes",
    "encode",
    "decode",
    "destring",
    "tostring",
    "insheet",
    "outsheet",
    "infile",
    "import",
    "export",
    "file",
    "log",
    "timer",
];

/// Scan `.ado` file content for dependency patterns.
/// Returns package names that appear to be required.
fn scan_content(content: &str) -> HashSet<String> {
    let mut deps = HashSet::new();

    for line in content.lines() {
        let trimmed = line.trim();
        // Skip comment lines
        if trimmed.starts_with('*') || trimmed.starts_with("//") {
            continue;
        }

        // Strip inline comments before matching
        let code = if let Some(pos) = trimmed.find("//") {
            &trimmed[..pos]
        } else {
            trimmed
        };

        for cap in REQUIRE_PATTERN.captures_iter(code) {
            if let Some(m) = cap.get(1) {
                deps.insert(m.as_str().to_lowercase());
            }
        }
        for cap in WHICH_PATTERN.captures_iter(code) {
            if let Some(m) = cap.get(1) {
                deps.insert(m.as_str().to_lowercase());
            }
        }
        for cap in FINDFILE_PATTERN.captures_iter(code) {
            if let Some(m) = cap.get(1) {
                deps.insert(m.as_str().to_lowercase());
            }
        }
    }

    deps
}

/// Scan `.ado` files in a directory for dependency patterns.
/// Returns package names that appear to be required.
pub fn scan_package_deps(package_dir: &Path) -> Vec<String> {
    let mut all_deps = HashSet::new();
    let builtins: HashSet<&str> = BUILTINS.iter().copied().collect();

    let entries = match std::fs::read_dir(package_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "ado") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                all_deps.extend(scan_content(&content));
            }
        }
    }

    // Filter out built-ins
    all_deps.retain(|name| !builtins.contains(name.as_str()));

    let mut result: Vec<String> = all_deps.into_iter().collect();
    result.sort();
    result
}

/// Compare scanned deps against installed packages.
/// Returns names that are detected but missing from the project.
pub fn find_missing_deps(
    package_name: &str,
    package_dir: &Path,
    installed: &HashSet<String>,
) -> Vec<String> {
    let deps = scan_package_deps(package_dir);
    let pkg_lower = package_name.to_lowercase();

    deps.into_iter()
        .filter(|dep| dep != &pkg_lower && !installed.contains(dep))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_require_pattern() {
        let content = "require ftools\nsome other code\n";
        let deps = scan_content(content);
        assert!(deps.contains("ftools"));
    }

    #[test]
    fn test_capture_require() {
        let content = "cap require reghdfe\n";
        let deps = scan_content(content);
        assert!(deps.contains("reghdfe"));
    }

    #[test]
    fn test_capture_full_require() {
        let content = "capture require moremata\n";
        let deps = scan_content(content);
        assert!(deps.contains("moremata"));
    }

    #[test]
    fn test_which_pattern() {
        let content = "which ftools\n";
        let deps = scan_content(content);
        assert!(deps.contains("ftools"));
    }

    #[test]
    fn test_cap_which_pattern() {
        let content = "cap which reghdfe\n";
        let deps = scan_content(content);
        assert!(deps.contains("reghdfe"));
    }

    #[test]
    fn test_which_with_ado_extension() {
        let content = "which ftools.ado\n";
        let deps = scan_content(content);
        assert!(deps.contains("ftools"));
    }

    #[test]
    fn test_findfile_pattern() {
        let content = "findfile \"moremata.hlp\"\n";
        let deps = scan_content(content);
        assert!(deps.contains("moremata"));
    }

    #[test]
    fn test_findfile_ado() {
        let content = "findfile \"ftools.ado\"\n";
        let deps = scan_content(content);
        assert!(deps.contains("ftools"));
    }

    #[test]
    fn test_findfile_mata() {
        let content = "findfile \"moremata.mata\"\n";
        let deps = scan_content(content);
        assert!(deps.contains("moremata"));
    }

    #[test]
    fn test_comments_skipped() {
        let content = "* require ftools\n// which reghdfe\nrequire real_dep\n";
        let deps = scan_content(content);
        assert!(!deps.contains("ftools"));
        assert!(!deps.contains("reghdfe"));
        assert!(deps.contains("real_dep"));
    }

    #[test]
    fn test_case_insensitive() {
        let content = "REQUIRE Ftools\nWhich RegHDFE\n";
        let deps = scan_content(content);
        assert!(deps.contains("ftools"));
        assert!(deps.contains("reghdfe"));
    }

    #[test]
    fn test_builtins_filtered() {
        let content = "require stata\nwhich mata\nrequire ftools\n";
        let deps = scan_content(content);
        // scan_content doesn't filter builtins, scan_package_deps does
        assert!(deps.contains("stata"));
        assert!(deps.contains("ftools"));
    }

    #[test]
    fn test_self_reference_filtered() {
        let installed: HashSet<String> = HashSet::new();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("mypkg.ado"),
            "require mypkg\nrequire otherpkg\n",
        )
        .unwrap();

        let missing = find_missing_deps("mypkg", dir.path(), &installed);
        assert!(!missing.contains(&"mypkg".to_string()));
        assert!(missing.contains(&"otherpkg".to_string()));
    }

    #[test]
    fn test_installed_deps_not_reported() {
        let mut installed = HashSet::new();
        installed.insert("ftools".to_string());

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pkg.ado"),
            "require ftools\nrequire missing\n",
        )
        .unwrap();

        let missing = find_missing_deps("pkg", dir.path(), &installed);
        assert!(!missing.contains(&"ftools".to_string()));
        assert!(missing.contains(&"missing".to_string()));
    }

    #[test]
    fn test_inline_comment_stripped() {
        let content = "some code // require fakepkg\nrequire realpkg\n";
        let deps = scan_content(content);
        assert!(!deps.contains("fakepkg"));
        assert!(deps.contains("realpkg"));
    }

    #[test]
    fn test_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let deps = scan_package_deps(dir.path());
        assert!(deps.is_empty());
    }

    #[test]
    fn test_nonexistent_dir() {
        let deps = scan_package_deps(Path::new("/nonexistent/path"));
        assert!(deps.is_empty());
    }
}
