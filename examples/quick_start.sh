#!/bin/bash
# FakeKey Quick Start Example
# This script demonstrates basic FakeKey usage

set -euo pipefail

echo "🚀 FakeKey Quick Start Example"
echo "================================"
echo ""

# Check if fakekey is installed
if ! command -v fakekey &> /dev/null; then
    echo "❌ FakeKey is not installed!"
    echo ""
    echo "Install FakeKey:"
    echo "  # Method 1: Cargo (recommended)"
    echo "  cargo install fakekey"
    echo ""
    echo "  # Method 2: Install script"
    echo "  curl -fsSL https://raw.githubusercontent.com/happyvibing/fakekey/main/install.sh | bash"
    echo ""
    echo "  # Method 3: Download binary"
    echo "  # Visit: https://github.com/happyvibing/fakekey/releases"
    exit 1
fi

echo "✅ FakeKey is installed: $(fakekey --version 2>/dev/null || echo 'version unknown')"
echo ""

# Show current status
echo "📊 Current Status:"
fakekey status
echo ""

# If not running, offer to start
if ! fakekey status | grep -q "RUNNING"; then
    echo "🔧 FakeKey is not running. Let's start it..."
    echo ""
    
    # Check if initialized
    if [ ! -d "$HOME/.fakekey" ]; then
        echo "📁 First time setup required. Initializing..."
        fakekey init
        echo ""
        echo "✅ Initialization complete!"
        echo ""
        echo "💡 Next steps:"
        echo "  1. Add an API key: fakekey add --name my-openai-key --key 'sk-...' --template openai"
        echo "  2. Start the proxy: fakekey start --daemon"
        echo "  3. Or run interactive setup: fakekey onboard"
        echo ""
        echo "🎯 For this example, let's run the interactive setup:"
        echo "  fakekey onboard"
    else
        echo "🔄 Starting FakeKey in daemon mode..."
        fakekey start --daemon
        echo ""
        echo "✅ FakeKey is now running!"
        echo ""
        echo "📊 Status:"
        fakekey status
        echo ""
        echo "💡 To stop FakeKey: fakekey stop"
    fi
else
    echo "✅ FakeKey is already running!"
    echo ""
    echo "📊 Current configuration:"
    fakekey list
fi

echo ""
echo "🎉 Quick start complete!"
echo ""
echo "📚 Next steps:"
echo "  • Add API keys: fakekey add --name <name> --key '<real-key>' --template <template>"
echo "  • View commands: fakekey --help"
echo "  • View logs: fakekey logs"
echo "  • Check status: fakekey status"
echo ""
echo "🌐 Learn more:"
echo "  • Documentation: https://github.com/happyvibing/fakekey"
echo "  • Report issues: https://github.com/happyvibing/fakekey/issues"
