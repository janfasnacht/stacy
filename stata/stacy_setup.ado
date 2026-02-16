*! stacy_setup.ado - Download and install stacy binary
*! Part of stacy: Modern Stata Workflow Tool
*! Version: 0.1.0

/*
    Download and install the stacy binary from GitHub releases.

    Syntax:
        stacy_setup [, Version(string) Path(string) Force Quietly]

    Options:
        version(string) - Specific version to install (default: latest)
        path(string)    - Installation directory (default: ~/.local/bin)
        force           - Overwrite existing installation
        quietly         - Suppress output

    Returns:
        r(installed)    - 1 if installed successfully, 0 otherwise
        r(version)      - Version that was installed
        r(path)         - Path where binary was installed
        r(already)      - 1 if was already installed, 0 otherwise

    Example:
        * Install latest version
        stacy_setup

        * Install specific version
        stacy_setup, version("v0.1.0")

        * Install to custom location
        stacy_setup, path("/usr/local/bin")
*/

program define stacy_setup, rclass
    version 14.0
    syntax [, Version(string) Path(string) Force Quietly]

    * Check if already installed
    _stacy_find_binary
    local already_installed = r(found)
    local existing_path `"`r(binary)'"'

    if `already_installed' == 1 & "`force'" == "" {
        if "`quietly'" == "" {
            di as text "stacy is already installed at: `existing_path'"
            di as text ""
            di as text "To reinstall, use: stacy_setup, force"
        }
        return scalar installed = 0
        return scalar already = 1
        return local path `"`existing_path'"'
        exit 0
    }

    * Determine target path
    if "`path'" == "" {
        local home : env HOME
        if "`c(os)'" == "Windows" {
            local localappdata : env LOCALAPPDATA
            local path "`localappdata'\stacy"
        }
        else {
            local path "`home'/.local/bin"
        }
    }

    * Create directory if needed
    capture confirm file `"`path'/."'
    if _rc != 0 {
        if "`quietly'" == "" {
            di as text "Creating directory: `path'"
        }
        if "`c(os)'" == "Windows" {
            quietly shell mkdir "`path'" 2>nul
        }
        else {
            quietly shell mkdir -p "`path'"
        }
    }

    * Determine platform and architecture
    local platform ""
    local arch ""
    local ext ""

    if "`c(os)'" == "MacOSX" {
        local platform "apple-darwin"
        * Check architecture
        if "`c(machine_type)'" == "arm64" | "`c(processor)'" == "arm" {
            local arch "aarch64"
        }
        else {
            local arch "x86_64"
        }
        local ext ""
    }
    else if "`c(os)'" == "Unix" {
        local platform "unknown-linux-gnu"
        local arch "x86_64"
        local ext ""
    }
    else if "`c(os)'" == "Windows" {
        local platform "pc-windows-msvc"
        local arch "x86_64"
        local ext ".exe"
    }
    else {
        di as error "Unsupported operating system: `c(os)'"
        exit 198
    }

    * Determine version
    if "`version'" == "" {
        local version "v0.1.0"
    }

    * Construct download URL
    local basename "stacy-`version'-`arch'-`platform'"
    local filename "`basename'.tar.gz"
    if "`c(os)'" == "Windows" {
        local filename "`basename'.zip"
    }

    local base_url "https://github.com/janfasnacht/stacy/releases/download"
    local download_url "`base_url'/`version'/`filename'"

    if "`quietly'" == "" {
        di as text ""
        di as text "stacy Setup"
        di as text "==========="
        di as text ""
        di as text "Platform: `c(os)' (`arch')"
        di as text "Version:  `version'"
        di as text "Target:   `path'"
        di as text ""
        di as text "Downloading from: `download_url'"
    }

    * Download file
    tempfile download_file
    tempfile extract_dir

    if "`c(os)'" == "Windows" {
        * Use PowerShell on Windows
        quietly shell powershell -Command "Invoke-WebRequest -Uri '`download_url'' -OutFile '`download_file''"
    }
    else {
        * Use curl on Unix/macOS
        quietly shell curl -L -o "`download_file'" "`download_url'" 2>&1
    }

    * Check if download succeeded
    capture confirm file `"`download_file'"'
    if _rc != 0 {
        di as error "Failed to download stacy binary"
        di as error "URL: `download_url'"
        di as text ""
        di as text "Please check:"
        di as text "  1. Internet connection"
        di as text "  2. Version exists at github.com/janfasnacht/stacy/releases"
        return scalar installed = 0
        return scalar already = 0
        exit 601
    }

    * Extract archive
    if "`quietly'" == "" {
        di as text "Extracting..."
    }

    local target_binary "`path'/stacy`ext'"

    if "`c(os)'" == "Windows" {
        quietly shell powershell -Command "Expand-Archive -Path '`download_file'' -DestinationPath '`path'' -Force"
    }
    else {
        quietly shell tar -xzf "`download_file'" -C "`path'"
    }

    * Verify installation
    capture confirm file `"`target_binary'"'
    if _rc != 0 {
        di as error "Failed to extract stacy binary"
        return scalar installed = 0
        return scalar already = 0
        exit 601
    }

    * Make executable (Unix/macOS)
    if "`c(os)'" != "Windows" {
        quietly shell chmod +x "`target_binary'"
    }

    * Verify it works
    if "`quietly'" == "" {
        di as text "Verifying installation..."
    }

    tempfile ver_out
    quietly shell "`target_binary'" --version > "`ver_out'" 2>&1

    tempname fh
    local installed_version ""
    capture file open `fh' using "`ver_out'", read text
    if _rc == 0 {
        file read `fh' line
        file close `fh'
        local installed_version `"`line'"'
    }

    * Set global for future use
    global stacy_binary "`target_binary'"

    * Report success
    if "`quietly'" == "" {
        di as text ""
        di as result "stacy installed successfully!"
        di as text ""
        di as text "  Binary:  `target_binary'"
        di as text "  Version: `installed_version'"
        di as text ""
        di as text "The stacy binary is now available. You can run:"
        di as text "  stacy doctor    - Check system status"
        di as text "  stacy run       - Execute Stata scripts"
        di as text ""
        di as text "Note: Make sure `path' is in your PATH for command-line use."
    }

    return scalar installed = 1
    return scalar already = 0
    return local version `"`installed_version'"'
    return local path `"`target_binary'"'
end
