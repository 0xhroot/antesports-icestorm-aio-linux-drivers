#!/bin/bash
set -euo pipefail

echo "=== Building ANTESPORTS Monitor Linux ==="

# Build everything
cargo build --release "$@"

echo ""
echo "Build complete. Run with:"
echo "  sudo ./target/release/ant-monitor-daemon"
echo "  ./target/release/ant-monitor-gui"
echo ""
echo "To install system-wide: sudo ./scripts/install.sh"
