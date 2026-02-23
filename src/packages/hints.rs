//! Post-install hints for packages with known implicit dependencies
//!
//! Some Stata packages require other packages at runtime but don't declare
//! formal dependencies. This module provides curated hints to help users
//! install required companion packages.

/// Return a hint string for packages with known implicit dependencies.
///
/// Returns `None` for packages without known dependency issues.
pub fn get_hint(package: &str) -> Option<&'static str> {
    match package {
        "reghdfe" => Some("reghdfe requires ftools and require. Run: stacy add ftools require"),
        "ivreghdfe" => Some(
            "ivreghdfe requires reghdfe, ftools, and ivreg2. Run: stacy add reghdfe ftools ivreg2",
        ),
        "ppmlhdfe" => Some("ppmlhdfe requires reghdfe and ftools. Run: stacy add reghdfe ftools"),
        "avar" => {
            Some("avar is required by ivreg2 and ranktest. Usually installed as a dependency.")
        }
        "ranktest" => Some("ranktest requires avar. Run: stacy add avar"),
        "estout" => {
            Some("estout provides esttab. For publication tables, also consider: stacy add estadd")
        }
        "grstyle" => {
            Some("grstyle requires palettes and colrspace. Run: stacy add palettes colrspace")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_hints() {
        assert!(get_hint("reghdfe").is_some());
        assert!(get_hint("ivreghdfe").is_some());
        assert!(get_hint("ppmlhdfe").is_some());
        assert!(get_hint("avar").is_some());
        assert!(get_hint("ranktest").is_some());
        assert!(get_hint("estout").is_some());
        assert!(get_hint("grstyle").is_some());
    }

    #[test]
    fn test_unknown_package_no_hint() {
        assert!(get_hint("somepkg").is_none());
        assert!(get_hint("").is_none());
        assert!(get_hint("ftools").is_none());
    }

    #[test]
    fn test_hint_content() {
        let hint = get_hint("reghdfe").unwrap();
        assert!(hint.contains("ftools"));
        assert!(hint.contains("require"));
        assert!(hint.contains("stacy add"));

        let hint = get_hint("grstyle").unwrap();
        assert!(hint.contains("palettes"));
        assert!(hint.contains("colrspace"));
    }
}
