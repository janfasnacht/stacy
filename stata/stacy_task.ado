*! stacy_task.ado - Run tasks from stacy.toml
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 1.0.1
*! AUTO-GENERATED - DO NOT EDIT
*! Regenerate with: cargo xtask codegen

/*
    Run tasks from stacy.toml

    Syntax:
        stacy_task [task] [, options]

    Options:
        FROZEN               - Fail if lockfile doesn't match stacy.toml
        LIST                 - List available tasks

    Returns:
        r(duration_secs       ) - Total execution time in seconds (scalar)
        r(exit_code           ) - Exit code (0=success) (scalar)
        r(failed_count        ) - Number of failed scripts (scalar)
        r(script_count        ) - Number of scripts executed (scalar)
        r(success             ) - Whether task succeeded (1=yes, 0=no) (scalar)
        r(success_count       ) - Number of successful scripts (scalar)
        r(task_count          ) - Number of tasks defined (scalar)
        r(task_name           ) - Name of the task (local)
        r(task_names          ) - Comma-separated task names (for --list) (local)
*/

program define stacy_task, rclass
    version 14.0
    syntax [anything(name=task)] [, FROZEN LIST]

    * Build command arguments
    local cmd "task"

    if `"`task'"' != "" {
        local cmd `"`cmd' "`task'""'
    }

    if "`frozen'" != "" {
        local cmd `"`cmd' --frozen"'
    }

    if "`list'" != "" {
        local cmd `"`cmd' --list"'
    }

    * Execute via _stacy_exec
    _stacy_exec `cmd'
    local exec_rc = r(exit_code)

    * Map parsed values to r() returns
    capture confirm scalar stacy_duration_secs
    if _rc == 0 {
        return scalar duration_secs = scalar(stacy_duration_secs)
    }

    capture confirm scalar stacy_exit_code
    if _rc == 0 {
        return scalar exit_code = scalar(stacy_exit_code)
    }

    capture confirm scalar stacy_failed_count
    if _rc == 0 {
        return scalar failed_count = scalar(stacy_failed_count)
    }

    capture confirm scalar stacy_script_count
    if _rc == 0 {
        return scalar script_count = scalar(stacy_script_count)
    }

    capture confirm scalar stacy_success
    if _rc == 0 {
        return scalar success = scalar(stacy_success)
    }

    capture confirm scalar stacy_success_count
    if _rc == 0 {
        return scalar success_count = scalar(stacy_success_count)
    }

    capture confirm scalar stacy_task_count
    if _rc == 0 {
        return scalar task_count = scalar(stacy_task_count)
    }

    if `"${stacy_task_name}"' != "" {
        return local task_name `"${stacy_task_name}"'
    }

    if `"${stacy_task_names}"' != "" {
        return local task_names `"${stacy_task_names}"'
    }

    * Return failure if command failed
    if `exec_rc' != 0 {
        exit `exec_rc'
    }
end
