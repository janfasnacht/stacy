#!/bin/bash
# stacy installer for macOS and Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/janfasnacht/stacy/main/install.sh | bash

set -e

REPO="janfasnacht/stacy"
INSTALL_DIR="${STACY_INSTALL_DIR:-$HOME/.local/bin}"

# Colors (disable if not terminal)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[0;33m'
    NC='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    NC=''
fi

info() { echo -e "${GREEN}==>${NC} $1"; }
warn() { echo -e "${YELLOW}warning:${NC} $1"; }
error() { echo -e "${RED}error:${NC} $1" >&2; exit 1; }

# Detect OS and architecture
detect_platform() {
    local os arch

    case "$(uname -s)" in
        Darwin) os="apple-darwin" ;;
        Linux)  os="unknown-linux-gnu" ;;
        *)      error "Unsupported OS: $(uname -s). Use Windows installer or build from source." ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)  arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        *)             error "Unsupported architecture: $(uname -m)" ;;
    esac

    echo "${arch}-${os}"
}

# Get latest release version from GitHub
get_latest_version() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" |
        grep '"tag_name"' |
        sed -E 's/.*"([^"]+)".*/\1/'
}

# Download and install
install_stacy() {
    local platform="$1"
    local version="$2"
    local url="https://github.com/${REPO}/releases/download/${version}/stacy-${version}-${platform}.tar.gz"
    local tmp_dir

    tmp_dir=$(mktemp -d)
    trap 'rm -rf "'"$tmp_dir"'"' EXIT

    info "Downloading stacy ${version} for ${platform}..."
    if ! curl -fsSL "$url" -o "$tmp_dir/stacy.tar.gz"; then
        error "Failed to download from $url"
    fi

    info "Extracting..."
    tar -xzf "$tmp_dir/stacy.tar.gz" -C "$tmp_dir"

    info "Installing to ${INSTALL_DIR}..."
    mkdir -p "$INSTALL_DIR"
    mv "$tmp_dir/stacy" "$INSTALL_DIR/stacy"
    chmod +x "$INSTALL_DIR/stacy"

    info "Installed stacy ${version} to ${INSTALL_DIR}/stacy"
}

# Check if install dir is in PATH
check_path() {
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        echo ""
        warn "$INSTALL_DIR is not in your PATH"
        echo ""
        echo "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo ""
        echo "    export PATH=\"$INSTALL_DIR:\$PATH\""
        echo ""
    fi
}

main() {
    info "stacy installer"
    echo ""

    # Check for curl
    if ! command -v curl &> /dev/null; then
        error "curl is required but not installed"
    fi

    local platform version

    platform=$(detect_platform)
    info "Detected platform: ${platform}"

    version=$(get_latest_version)
    if [ -z "$version" ]; then
        error "Could not determine latest version. Check https://github.com/${REPO}/releases"
    fi
    info "Latest version: ${version}"

    install_stacy "$platform" "$version"
    check_path

    echo ""
    info "Done! Run 'stacy --help' to get started."
}

main "$@"
