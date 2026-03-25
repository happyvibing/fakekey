#!/bin/bash
# FakeKey installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/happyvibing/fakekey/main/install.sh | bash

set -euo pipefail

REPO="happyvibing/fakekey"
INSTALL_DIR="${FAKEKEY_INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="fakekey"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() { echo -e "${BLUE}==>${NC} $1"; }
success() { echo -e "${GREEN}==>${NC} $1"; }
warn() { echo -e "${YELLOW}==>${NC} $1"; }
error() { echo -e "${RED}Error:${NC} $1" >&2; exit 1; }

detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="macos" ;;
        MINGW*|MSYS*|CYGWIN*) os="windows" ;;
        *) error "Unsupported OS: $(uname -s)" ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)  arch="amd64" ;;
        aarch64|arm64) arch="arm64" ;;
        *) error "Unsupported architecture: $(uname -m)" ;;
    esac

    echo "${os}-${arch}"
}

get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    if command -v curl &>/dev/null; then
        curl -fsSL "$url" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/'
    elif command -v wget &>/dev/null; then
        wget -qO- "$url" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/'
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
}

download() {
    local url="$1" dest="$2"
    if command -v curl &>/dev/null; then
        curl -fsSL "$url" -o "$dest"
    elif command -v wget &>/dev/null; then
        wget -qO "$dest" "$url"
    fi
}

verify_checksum() {
    local file="$1" expected="$2"
    local actual
    if command -v sha256sum &>/dev/null; then
        actual=$(sha256sum "$file" | awk '{print $1}')
    elif command -v shasum &>/dev/null; then
        actual=$(shasum -a 256 "$file" | awk '{print $1}')
    else
        warn "Cannot verify checksum: sha256sum/shasum not found"
        return 0
    fi

    if [ "$actual" != "$expected" ]; then
        error "Checksum verification failed!\n  Expected: ${expected}\n  Actual:   ${actual}"
    fi
}

main() {
    local version="${1:-}"
    if [ "$version" = "--dry-run" ]; then
        version="${2:-}"
        DRY_RUN=true
    fi
    
    info "Detecting platform..."
    local platform
    platform=$(detect_platform)
    info "Platform: ${platform}"

    if [ -z "$version" ]; then
        info "Fetching latest version..."
        version=$(get_latest_version)
    fi

    if [ -z "$version" ]; then
        error "Could not determine version. Specify one: $0 v0.1.0"
    fi
    info "Version: ${version}"

    local archive_name="fakekey-${platform}"
    local ext="tar.gz"
    if [[ "$platform" == windows-* ]]; then
        ext="zip"
    fi
    local archive_file="${archive_name}.${ext}"

    local download_url="https://github.com/${REPO}/releases/download/${version}/${archive_file}"
    local checksums_url="https://github.com/${REPO}/releases/download/${version}/checksums-sha256.txt"

    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap 'rm -rf "$tmp_dir"' EXIT

    info "Downloading ${archive_file}..."
    download "$download_url" "${tmp_dir}/${archive_file}"

    # Verify checksum
    info "Verifying checksum..."
    if download "$checksums_url" "${tmp_dir}/checksums-sha256.txt" 2>/dev/null; then
        local expected_hash
        expected_hash=$(grep "${archive_file}" "${tmp_dir}/checksums-sha256.txt" | awk '{print $1}')
        if [ -n "$expected_hash" ]; then
            verify_checksum "${tmp_dir}/${archive_file}" "$expected_hash"
            success "Checksum verified"
        else
            warn "Checksum not found for ${archive_file}, skipping verification"
        fi
    else
        warn "Could not download checksums, skipping verification"
    fi

    # Extract
    info "Extracting..."
    if [ "$ext" = "tar.gz" ]; then
        tar xzf "${tmp_dir}/${archive_file}" -C "${tmp_dir}"
    else
        unzip -q "${tmp_dir}/${archive_file}" -d "${tmp_dir}"
    fi

    # Install
    info "Installing to ${INSTALL_DIR}..."
    if [ -w "$INSTALL_DIR" ]; then
        mv "${tmp_dir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    else
        warn "Requires sudo to install to ${INSTALL_DIR}"
        sudo mv "${tmp_dir}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"
    fi
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

    # Verify installation
    if command -v fakekey &>/dev/null; then
        local installed_version
        installed_version=$(fakekey --version 2>/dev/null || echo "unknown")
        success "FakeKey installed successfully! (${installed_version})"
    else
        success "FakeKey installed to ${INSTALL_DIR}/${BINARY_NAME}"
        warn "${INSTALL_DIR} may not be in your PATH"
    fi

    echo ""
    info "Quick start:"
    echo "  fakekey onboard          # Interactive setup wizard"
    echo "  fakekey --help           # Show all commands"
    echo ""
}

main "$@"
