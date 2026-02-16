# stacy installer for Windows
# Usage: irm https://raw.githubusercontent.com/janfasnacht/stacy/main/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "janfasnacht/stacy"
$InstallDir = if ($env:STACY_INSTALL_DIR) { $env:STACY_INSTALL_DIR } else { "$env:LOCALAPPDATA\stacy" }

function Write-Info { param($Message) Write-Host "==> " -ForegroundColor Green -NoNewline; Write-Host $Message }
function Write-Warn { param($Message) Write-Host "warning: " -ForegroundColor Yellow -NoNewline; Write-Host $Message }
function Write-Err { param($Message) Write-Host "error: " -ForegroundColor Red -NoNewline; Write-Host $Message; exit 1 }

function Get-LatestVersion {
    $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    return $response.tag_name
}

function Install-Stacy {
    param($Version)

    $platform = "x86_64-pc-windows-msvc"
    $url = "https://github.com/$Repo/releases/download/$Version/stacy-$Version-$platform.zip"
    $tempDir = Join-Path $env:TEMP "stacy-install"
    $zipPath = Join-Path $tempDir "stacy.zip"

    # Cleanup and create temp dir
    if (Test-Path $tempDir) { Remove-Item -Recurse -Force $tempDir }
    New-Item -ItemType Directory -Path $tempDir | Out-Null

    Write-Info "Downloading stacy $Version..."
    try {
        Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing
    } catch {
        Write-Err "Failed to download from $url"
    }

    Write-Info "Extracting..."
    Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force

    Write-Info "Installing to $InstallDir..."
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir | Out-Null
    }

    Move-Item -Path (Join-Path $tempDir "stacy.exe") -Destination (Join-Path $InstallDir "stacy.exe") -Force

    # Cleanup
    Remove-Item -Recurse -Force $tempDir

    Write-Info "Installed stacy $Version to $InstallDir\stacy.exe"
}

function Add-ToPath {
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$InstallDir*") {
        Write-Host ""
        Write-Warn "$InstallDir is not in your PATH"
        Write-Host ""

        $addToPath = Read-Host "Add to PATH? (Y/n)"
        if ($addToPath -ne "n" -and $addToPath -ne "N") {
            [Environment]::SetEnvironmentVariable("Path", "$currentPath;$InstallDir", "User")
            $env:Path = "$env:Path;$InstallDir"
            Write-Info "Added to PATH. Restart your terminal for changes to take effect."
        } else {
            Write-Host ""
            Write-Host "To add manually, run:"
            Write-Host ""
            Write-Host "    `$env:Path += `";$InstallDir`""
            Write-Host ""
        }
    }
}

function Main {
    Write-Info "stacy installer for Windows"
    Write-Host ""

    $version = Get-LatestVersion
    if (-not $version) {
        Write-Err "Could not determine latest version. Check https://github.com/$Repo/releases"
    }
    Write-Info "Latest version: $version"

    Install-Stacy -Version $version
    Add-ToPath

    Write-Host ""
    Write-Info "Done! Run 'stacy --help' to get started."
}

Main
