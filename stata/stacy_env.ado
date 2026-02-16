*! stacy_env.ado - Show environment configuration
*! Part of stacy: Modern Stata Workflow Tool
*! Version: 0.1.0
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
    capture confirm scalar _stacy_json_adopath_count
    if _rc == 0 {
        return scalar adopath_count = scalar(_stacy_json_adopath_count)
    }

    capture confirm scalar _stacy_json_has_config
    if _rc == 0 {
        return scalar has_config = scalar(_stacy_json_has_config)
    }

    capture confirm scalar _stacy_json_show_progress
    if _rc == 0 {
        return scalar show_progress = scalar(_stacy_json_show_progress)
    }

    if `"`_stacy_json_cache_dir'"' != "" {
        return local cache_dir `"`_stacy_json_cache_dir'"'
    }

    if `"`_stacy_json_log_dir'"' != "" {
        return local log_dir `"`_stacy_json_log_dir'"'
    }

    if `"`_stacy_json_project_root'"' != "" {
        return local project_root `"`_stacy_json_project_root'"'
    }

    if `"`_stacy_json_stata_binary'"' != "" {
        return local stata_binary `"`_stacy_json_stata_binary'"'
    }

    if `"`_stacy_json_stata_source'"' != "" {
        return local stata_source `"`_stacy_json_stata_source'"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
