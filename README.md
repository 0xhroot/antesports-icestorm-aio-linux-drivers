# ANT Esports IceStorm AIO — Linux Driver

Native Linux driver for ANT Esports IceStorm AIO LCD coolers (and compatible
Vevor / HT LCD coolers using the same USB HID protocol). No Windows, no WINE.

## Supported Hardware

| Device | VID:PID | Protocol |
|--------|---------|----------|
| ANT Esports IceStorm AIO LCD | `0x5131:0x2007` | Classic |
| Vevor / HT LCD Coolers | `0x5131:0x2007` | Classic |
| Antec Vortex View / Flux Pro | `0x2022:0x0522` | iUnity |

## Quick Start

```bash
# Prerequisites
sudo apt install libudev-dev libhidapi-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build
git clone <your-repo-url>
cd ant-esports-icestorm-aio-linux-drivers
cargo build --release

# Run (no install)
./target/release/ant-monitor-daemon

# Or install system-wide
sudo ./scripts/install.sh
```

## Architecture

```
├── protocol/          # HID frame construction & value encoding
├── sensors/           # Linux hwmon/sysfs sensor reading
├── usb/               # HID device discovery & communication
├── ant-monitor-daemon # systemd service (headless, 200ms loop)
├── ant-monitor-gui    # Terminal-based live sensor display
└── gui/               # Qt6 desktop GUI (optional)
```

Sensor data is read from Linux kernel interfaces (`/sys/class/hwmon`,
`/proc/stat`, `/proc/meminfo`), encoded into a 65-byte HID output report,
and sent to the LCD every 200ms. The LCD firmware renders everything —
the host only streams numeric values.

## Install

```bash
sudo ./scripts/install.sh
```

This installs the daemon, udev rules, and systemd service. Replug the USB
cooler after install for udev rules to take effect.

### Manual

```bash
sudo install -m 755 target/release/ant-monitor-daemon /usr/bin/
sudo install -m 755 target/release/ant-monitor-gui /usr/bin/
sudo install -m 644 scripts/99-ant-monitor.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
sudo install -m 644 systemd/ant-monitor-daemon.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now ant-monitor-daemon
```

## Usage

```bash
systemctl status ant-monitor-daemon
journalctl -u ant-monitor-daemon -f
ant-monitor-gui
```

## Protocol

See [docs/PROTOCOL.md](docs/PROTOCOL.md) for the full reverse-engineered
USB HID protocol specification.

## License

MIT
