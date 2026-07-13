//! Post-install dependency scanning for Stata packages
//!
//! Scans a package's `.ado` files for dependency patterns (`require`, `which`,
//! `findfile`) and reports any that are missing from the project's config.
//! This catches implicit dependencies that would otherwise only surface at
//! runtime when Stata errors out.
//!
//! References to files the package ships itself are internal, not dependencies,
//! so they are excluded — see [`provided_names`].

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

/// File extensions that let a package define a name its other files can refer
/// to: commands (`.ado`), Mata source and libraries, classes, dialogs, schemes
/// and help files. A reference to one of these is internal to the package.
const PROVIDED_EXTENSIONS: &[&str] = &[
    "ado", "class", "dlg", "do", "hlp", "idlg", "ihlp", "mata", "mlib", "mnu", "mo", "plugin",
    "scheme", "smcl", "sthlp", "style",
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

/// Names the package provides itself: its own name plus the stem of every
/// shipped file that can define a name.
///
/// A package's `.ado` files routinely load the package's own Mata source,
/// classes and sub-commands by file name — `ftools` ships `fcollapse_main.mata`
/// and `ftools_type_aliases.mata` and loads both with `findfile`. Those are
/// internal files, not external packages, so they must never be reported as
/// missing dependencies.
pub fn provided_names(package_name: &str, package_dir: &Path) -> HashSet<String> {
    let mut names = HashSet::new();
    names.insert(package_name.to_lowercase());

    let entries = match std::fs::read_dir(package_dir) {
        Ok(e) => e,
        Err(_) => return names,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let provides_name = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| PROVIDED_EXTENSIONS.contains(&ext.to_lowercase().as_str()));
        if !provides_name {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let name = file_name.to_lowercase();

        // Both the full stem and the leading component, so a file shipped as
        // `parallel_map_template.do.ado` is provided under either spelling.
        if let Some(stem) = name.rsplit_once('.').map(|(stem, _)| stem) {
            names.insert(stem.to_string());
        }
        if let Some((head, _)) = name.split_once('.') {
            names.insert(head.to_string());
        }
    }

    names
}

/// Compare scanned deps against the names the package provides itself and the
/// packages already installed. Returns names that are detected but missing.
pub fn find_missing_deps(
    package_name: &str,
    package_dir: &Path,
    installed: &HashSet<String>,
) -> Vec<String> {
    let deps = scan_package_deps(package_dir);
    let provided = provided_names(package_name, package_dir);

    deps.into_iter()
        .filter(|dep| !provided.contains(dep) && !installed.contains(dep))
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

    /// Issue #101: `ftools` loads its own Mata source with `findfile`. Those
    /// files ship inside the package, so they are not missing dependencies.
    #[test]
    fn test_self_shipped_mata_not_reported() {
        let installed: HashSet<String> = HashSet::new();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("fcollapse.ado"),
            "findfile \"ftools.mata\"\nfindfile \"fcollapse_main.mata\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("local_inlist.ado"),
            "findfile \"ftools_type_aliases.mata\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("ftools.mata"), "// mata source").unwrap();
        std::fs::write(dir.path().join("fcollapse_main.mata"), "// mata source").unwrap();
        std::fs::write(
            dir.path().join("ftools_type_aliases.mata"),
            "// mata source",
        )
        .unwrap();

        let missing = find_missing_deps("ftools", dir.path(), &installed);
        assert!(
            missing.is_empty(),
            "expected no missing deps, got {missing:?}"
        );
    }

    /// The self-provided set covers every file type a package ships that can
    /// define a name, not just `.ado`.
    #[test]
    fn test_self_shipped_non_ado_files_not_reported() {
        let installed: HashSet<String> = HashSet::new();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pkg.ado"),
            "findfile \"pkg_helper.class\"\nfindfile \"pkg_lib.mlib\"\n\
             findfile \"pkg_setup.do\"\nwhich pkg_sub\nrequire otherpkg\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("pkg_helper.class"), "// class").unwrap();
        std::fs::write(dir.path().join("pkg_lib.mlib"), "// mlib").unwrap();
        std::fs::write(dir.path().join("pkg_setup.do"), "// do").unwrap();
        std::fs::write(dir.path().join("pkg_sub.ado"), "// sub-command").unwrap();

        let missing = find_missing_deps("pkg", dir.path(), &installed);
        assert_eq!(missing, vec!["otherpkg".to_string()]);
    }

    /// A file shipped with a compound extension is provided under either
    /// spelling (`parallel_map_template.do.ado` in ftools).
    #[test]
    fn test_compound_extension_provided() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("parallel_map_template.do.ado"), "// tpl").unwrap();

        let provided = provided_names("ftools", dir.path());
        assert!(provided.contains("parallel_map_template"));
        assert!(provided.contains("parallel_map_template.do"));
    }

    /// A reference to a package that is not shipped is still reported.
    #[test]
    fn test_external_dep_still_reported() {
        let installed: HashSet<String> = HashSet::new();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pkg.ado"),
            "findfile \"pkg_main.mata\"\nrequire moremata\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("pkg_main.mata"), "// mata source").unwrap();

        let missing = find_missing_deps("pkg", dir.path(), &installed);
        assert_eq!(missing, vec!["moremata".to_string()]);
    }

    /// Data files a package ships do not define names, so they do not mask a
    /// dependency that happens to share their stem.
    #[test]
    fn test_data_files_do_not_provide_names() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("moremata.dta"), "// sample data").unwrap();

        let provided = provided_names("pkg", dir.path());
        assert!(!provided.contains("moremata"));
    }

    #[test]
    fn test_provided_names_includes_package_name() {
        let dir = tempfile::tempdir().unwrap();
        let provided = provided_names("MyPkg", dir.path());
        assert!(provided.contains("mypkg"));
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
