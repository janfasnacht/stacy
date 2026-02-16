*! stacy_env.ado - Show environment configuration
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.0.1
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Show environment configuration

    Syntax:
        stacy_env 

    Returns:
        r(adopath_count       ) - Number of adopath entries (scalar)
        r(has_config          ) - stacy.toml exists (1=yes, 0=no) (scalar)
        r(show_progress       ) - Progress shown (1=yes, 0=no) (scalar)
        r(cache_dir           ) - Global package cache directory (local)
        r(log_dir             ) - Project log directory (local)
        r(project_root        ) - Project root directory (local)
        r(stata_binary        ) - Path to Stata binary (local)
        r(stata_source        ) - How binary was detected (local)
*/

program define stacy_env, rclass
    version 14.0
    syntax 

    * Build command arguments
    local cmd "env"

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar stacy_adopath_count
    if _rc == 0 {
        return scalar adopath_count = scalar(stacy_adopath_count)
    }

    capture confirm scalar stacy_has_config
    if _rc == 0 {
        return scalar has_config = scalar(stacy_has_config)
    }

    capture confirm scalar stacy_show_progress
    if _rc == 0 {
        return scalar show_progress = scalar(stacy_show_progress)
    }

    if `"${stacy_cache_dir}"' != "" {
        return local cache_dir `"${stacy_cache_dir}"'
    }

    if `"${stacy_log_dir}"' != "" {
        return local log_dir `"${stacy_log_dir}"'
    }

    if `"${stacy_project_root}"' != "" {
        return local project_root `"${stacy_project_root}"'
    }

    if `"${stacy_stata_binary}"' != "" {
        return local stata_binary `"${stacy_stata_binary}"'
    }

    if `"${stacy_stata_source}"' != "" {
        return local stata_source `"${stacy_stata_source}"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
