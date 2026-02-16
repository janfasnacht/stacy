*! _stacy_find_binary.ado - Locate stacy binary
*! Part of stacy: Reproducible Stata Workflow Tool
*! Version: 0.1.0

/*
    Find the stacy binary path.

    Syntax: _stacy_find_binary [, path(local_name)]

    Search order:
    1. Global macro $stacy_binary (user override)
    2. PATH (via `which stacy`)
    3. Common installation locations

    Returns path in local macro (default: _stacy_binary_path)
    Sets r(found) = 1 if found, 0 otherwise
    Sets r(binary) to the path if found
*/

program define _stacy_find_binary, rclass
    version 14.0
    syntax [, path(name)]

    if "`path'" == "" {
        local path "_stacy_binary_path"
    }

    * 1. Check global override
    if `"$stacy_binary"' != "" {
        capture confirm file `"$stacy_binary"'
        if _rc == 0 {
            c_local `path' `"$stacy_binary"'
            return scalar found = 1
            return local binary `"$stacy_binary"'
            exit
        }
    }

    * 2. Check PATH via shell
    tempfile which_result
    quietly {
        if "`c(os)'" == "Windows" {
            shell where stacy > "`which_result'" 2>&1
        }
        else {
            shell which stacy > "`which_result'" 2>&1
        }
    }

    * Read result
    tempname fh
    capture file open `fh' using `"`which_result'"', read text
    if _rc == 0 {
        file read `fh' line
        file close `fh'
        local line = strtrim(`"`line'"')
        if `"`line'"' != "" & strpos(`"`line'"', "not found") == 0 & strpos(`"`line'"', "Could not find") == 0 {
            capture confirm file `"`line'"'
            if _rc == 0 {
                c_local `path' `"`line'"'
                return scalar found = 1
                return local binary `"`line'"'
                exit
            }
        }
    }

    * 3. Check common locations
    local locations ""
    if "`c(os)'" == "MacOSX" {
        local locations "/usr/local/bin/stacy /opt/homebrew/bin/stacy"
        local locations "`locations' ~/bin/stacy ~/.local/bin/stacy"
    }
    else if "`c(os)'" == "Unix" {
        local locations "/usr/local/bin/stacy /usr/bin/stacy"
        local locations "`locations' ~/bin/stacy ~/.local/bin/stacy"
    }
    else if "`c(os)'" == "Windows" {
        local localappdata : env LOCALAPPDATA
        local locations "`localappdata'\stacy\stacy.exe"
        local locations "`locations' C:\Program Files\stacy\stacy.exe"
        local locations "`locations' C:\stacy\stacy.exe"
    }

    foreach loc of local locations {
        * Expand tilde
        if substr(`"`loc'"', 1, 1) == "~" {
            local home : env HOME
            local loc = "`home'" + substr(`"`loc'"', 2, .)
        }
        capture confirm file `"`loc'"'
        if _rc == 0 {
            c_local `path' `"`loc'"'
            return scalar found = 1
            return local binary `"`loc'"'
            exit
        }
    }

    * Not found
    c_local `path' ""
    return scalar found = 0
    return local binary ""
end
