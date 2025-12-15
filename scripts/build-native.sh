#!/bin/bash

# Build script for protobuf.js native Rust module
# This script handles building the native module with proper error handling

set -e  # Exit on error

echo "=== Building protobuf.js Native Module ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: Rust/Cargo not found${NC}"
    echo ""
    echo "Please install Rust from https://rustup.rs/"
    echo "Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo ""
    echo "After installation, run: source \$HOME/.cargo/env"
    exit 1
fi

echo -e "${GREEN}✓${NC} Rust found: $(cargo --version)"
echo -e "${GREEN}✓${NC} Rustc found: $(rustc --version)"
echo ""

# Navigate to native directory
cd "$(dirname "$0")/../native" || exit 1

# Detect platform
OS=$(uname -s)
case "$OS" in
    Linux*)
        PLATFORM="Linux"
        EXT="so"
        ;;
    Darwin*)
        PLATFORM="macOS"
        EXT="dylib"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        PLATFORM="Windows"
        EXT="dll"
        ;;
    *)
        echo -e "${YELLOW}Warning: Unknown platform $OS${NC}"
        PLATFORM="Unknown"
        EXT="so"
        ;;
esac

echo -e "${GREEN}Platform:${NC} $PLATFORM"
echo ""

# Build mode (default: release)
BUILD_MODE="${1:-release}"

if [ "$BUILD_MODE" = "debug" ]; then
    echo "Building in DEBUG mode..."
    cargo build
    BUILD_DIR="debug"
else
    echo "Building in RELEASE mode (optimized)..."
    cargo build --release
    BUILD_DIR="release"
fi

echo ""

# Check if build succeeded
if [ ! -f "target/$BUILD_DIR/libprotobufjs_native.$EXT" ]; then
    echo -e "${RED}Error: Build failed - binary not found${NC}"
    exit 1
fi

echo -e "${GREEN}✓${NC} Build successful!"
echo ""

# Copy binary to expected location
echo "Copying binary to native/index.node..."
cp "target/$BUILD_DIR/libprotobufjs_native.$EXT" "index.node"

if [ -f "index.node" ]; then
    SIZE=$(ls -lh index.node | awk '{print $5}')
    echo -e "${GREEN}✓${NC} Binary copied successfully (size: $SIZE)"
else
    echo -e "${RED}Error: Failed to copy binary${NC}"
    exit 1
fi

echo ""

# Test the binary
echo "Testing native module..."
# Return to project root (we're currently in native/ directory)
cd ..
if node native/test.js > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Native module test passed!"
else
    echo -e "${YELLOW}Warning: Native module test failed${NC}"
    echo "The module may still work, but there might be compatibility issues."
fi

echo ""
echo -e "${GREEN}=== Build Complete ===${NC}"
echo ""
echo "The native module is ready to use."
echo "Location: native/index.node"
echo ""
echo "To test it, run:"
echo "  node native/test.js"
echo ""
echo "To use it in your application:"
echo "  var protobuf = require('protobufjs');"
echo ""
