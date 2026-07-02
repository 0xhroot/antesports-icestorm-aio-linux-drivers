#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "=== ANTESPORTS Monitor Linux Daemon Installer ==="

# 1. Build
echo "[1/4] Building project..."
cd "$PROJECT_DIR"
cargo build --release

# 2. Install binary
echo "[2/4] Installing binary..."
sudo install -m 755 target/release/ant-monitor-daemon /usr/bin/
sudo install -m 755 target/release/ant-monitor-gui /usr/bin/

# 3. Install udev rules
echo "[3/4] Installing udev rules..."
sudo install -m 644 scripts/99-ant-monitor.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger

# 4. Install systemd service
echo "[4/4] Installing systemd service..."
sudo install -m 644 systemd/ant-monitor-daemon.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable ant-monitor-daemon.service
sudo systemctl start ant-monitor-daemon.service

echo ""
echo "=== Installation complete ==="
echo "The daemon is now running. Check status with:"
echo "  systemctl status ant-monitor-daemon"
echo ""
echo "View logs with:"
echo "  journalctl -u ant-monitor-daemon -f"
echo ""
echo "If you need to unplug and replug the USB cable for udev rules to take effect."
echo "Run the GUI with: ant-monitor-gui"
