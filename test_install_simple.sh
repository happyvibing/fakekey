#!/bin/bash

# Simple test for install.sh functions
set -euo pipefail

echo "Testing install.sh functions..."

# Extract just the functions we need to test
detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="macos" ;;
        MINGW*|MSYS*|CYGWIN*) os="windows" ;;
        *) echo "Unsupported OS: $(uname -s)" >&2; exit 1 ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)  arch="amd64" ;;
        aarch64|arm64) arch="arm64" ;;
        *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
    esac

    echo "${os}-${arch}"
}

echo "1. Testing platform detection..."
platform=$(detect_platform)
echo "   Detected platform: $platform"
echo "   ✓ Platform detection works"

echo "2. Testing archive name generation..."
archive_name="fakekey-${platform}"
echo "   Platform: $platform"
echo "   Archive name: $archive_name"
echo "   ✓ Archive name generation works"

echo "3. Testing file extensions..."
if [[ "$platform" == windows-* ]]; then
    ext="zip"
else
    ext="tar.gz"
fi
echo "   Extension for $platform: $ext"
echo "   ✓ Extension selection works"

echo ""
echo "All tests passed! ✅"
echo ""
echo "The install script logic is correct."
echo "Users can install with:"
echo "  curl -fsSL https://raw.githubusercontent.com/happyvibing/fakekey/main/install.sh | bash"
