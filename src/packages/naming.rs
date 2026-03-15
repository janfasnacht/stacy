//! Package naming hints for SSC
//!
//! Many Stata commands are provided by packages with different names.
//! When a user tries to install a command name that isn't a package,
//! this module suggests the correct package.

/// Look up which package provides a given command name.
///
/// Returns `Some((package_name, commands))` where `commands` lists the
/// well-known commands that package provides.
pub fn find_provider(name: &str) -> Option<(&'static str, &'static [&'static str])> {
    // Linear scan is fine — the map is small and lookups only happen on 404.
    for &(package, commands) in COMMAND_TO_PACKAGE {
        if commands.contains(&name) {
            return Some((package, commands));
        }
    }
    None
}

/// Curated map: (package_name, &[command_names_it_provides])
///
/// Only includes cases where the command name differs from the package name.
/// Each entry maps a set of well-known commands to the SSC package that
/// provides them.
const COMMAND_TO_PACKAGE: &[(&str, &[&str])] = &[
    // labutil: label manipulation utilities
    ("labutil", &["labmask", "labvarch", "labvalch", "labcd"]),
    // estout: estimation output tables
    ("estout", &["esttab", "eststo", "estpost", "estadd"]),
    // ftools: fast Stata tools
    (
        "ftools",
        &[
            "fegen",
            "fcollapse",
            "fmerge",
            "fisid",
            "flevelsof",
            "fsort",
            "ftab",
        ],
    ),
    // gtools: fast group commands
    (
        "gtools",
        &[
            "gcollapse",
            "gegen",
            "gisid",
            "glevelsof",
            "gunique",
            "gdistinct",
            "gquantiles",
            "gstats",
            "hashsort",
            "greshape",
            "gduplicates",
            "gtop",
        ],
    ),
    // moremata: extended Mata functions
    ("moremata", &["mf_mm_quantile", "mf_mm_density"]),
    // palettes: color palettes
    (
        "palettes",
        &["colorpalette", "symbolpalette", "linepalette"],
    ),
    // boottest: wild bootstrap
    ("boottest", &["waldtest"]),
    // rdrobust: regression discontinuity
    ("rdrobust", &["rdplot", "rdbwselect"]),
    // ietoolkit: World Bank impact evaluation
    (
        "ietoolkit",
        &[
            "iefolder", "iedorep", "iebaltab", "ieddtab", "iegraph", "iesave",
        ],
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_provider_known() {
        let (pkg, commands) = find_provider("labmask").unwrap();
        assert_eq!(pkg, "labutil");
        assert!(commands.contains(&"labmask"));
    }

    #[test]
    fn test_find_provider_another() {
        let (pkg, _) = find_provider("esttab").unwrap();
        assert_eq!(pkg, "estout");
    }

    #[test]
    fn test_find_provider_unknown() {
        assert!(find_provider("notarealcommand").is_none());
    }

    #[test]
    fn test_find_provider_gtools() {
        let (pkg, _) = find_provider("gcollapse").unwrap();
        assert_eq!(pkg, "gtools");
    }

    #[test]
    fn test_find_provider_ftools() {
        let (pkg, _) = find_provider("fcollapse").unwrap();
        assert_eq!(pkg, "ftools");
    }

    #[test]
    fn test_find_provider_case_sensitive() {
        // Input should be lowercased before calling
        assert!(find_provider("LabMask").is_none());
    }

    #[test]
    fn test_no_duplicate_commands() {
        let mut seen = std::collections::HashSet::new();
        for &(pkg, commands) in COMMAND_TO_PACKAGE {
            for &cmd in commands {
                assert!(
                    seen.insert(cmd),
                    "Duplicate command '{}' (already mapped, now in '{}')",
                    cmd,
                    pkg
                );
            }
        }
    }

    #[test]
    fn test_map_well_formed() {
        for &(pkg, commands) in COMMAND_TO_PACKAGE {
            assert!(!pkg.is_empty(), "Package name must not be empty");
            assert!(
                !commands.is_empty(),
                "Commands list must not be empty for {}",
                pkg
            );
            // Commands matching the package name would never trigger a 404,
            // so they shouldn't be in the map.
            for &cmd in commands {
                assert_ne!(
                    cmd, pkg,
                    "Command '{}' equals package name — remove it (SSC will find it directly)",
                    cmd
                );
            }
        }
    }
}
