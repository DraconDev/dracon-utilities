# dracon-system

A deterministic system utility for proactive disk space monitoring and automatic cleanup. Designed to prevent "disk full" emergencies on development machines and servers.

## Features

### Disk Space Monitoring
- **Early Warning (70%)** - Proactive notification before space becomes critical
- **Warning (80%)** - State change notification
- **Action (90%)** - Automatic cleanup triggers
- **Critical (95%)** - Aggressive mitigation

### Automatic Rust Target Cleanup
- Automatically cleans `target/` directories when disk hits action level
- Smart protection for active builds:
  - Detects running `cargo`, `rustc`, `clippy-driver` processes
  - Protects target dirs in their working directories
  - Protects recently modified target dirs (configurable)
- Configurable minimum size threshold

### Build-Aware Monitoring
- Detects active Rust build processes
- Protects their target directories from cleanup
- Prevents breaking active compilation

### Disk Space Trend Prediction
- Tracks disk usage history over time
- Predicts when disk will fill based on usage rate
- Warns if disk predicted to fill within configurable hours

### Inode Monitoring
- Monitors inode usage on root filesystem
- Warns when inode usage exceeds threshold (default 85%)
- Critical for systems with many small files

### Zombie Process Detection
- Detects accumulated zombie processes
- Alerts when zombie count exceeds threshold (default 20)
- Helps identify parent processes not reaping children

### Large Log File Detection
- Scans configured directories for large log files
- Alerts on files exceeding size threshold (default 100 MiB)
- Helps identify runaway logging

## Installation

### Quick Install (User Service)

```bash
cd dracon-system
./install.sh
```

This will:
1. Build the release binary
2. Install to `~/.local/bin/dracon-system`
3. Set up systemd user service

### Manual Install

```bash
# Build
cargo build --release

# Copy binary
cp target/release/dracon-system ~/.local/bin/

# (Optional) Install systemd service
mkdir -p ~/.config/systemd/user
cp dracon-system-guard.service ~/.config/systemd/user/
systemctl --user daemon-reload
```

### Server Deployment (System-wide)

For servers, you may want to run as a system service:

```bash
# Build
cargo build --release

# Copy binary
sudo cp target/release/dracon-system /usr/local/bin/

# Create dedicated user (optional but recommended)
sudo useradd -r -s /bin/false dracon-guard

# Create system service file
sudo cat > /etc/systemd/system/dracon-system-guard.service << 'EOF'
[Unit]
Description=Dracon System Guard - Proactive disk space monitoring
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/dracon-system guard daemon
Restart=always
RestartSec=10
User=root
# Or use dedicated user with appropriate permissions
# User=dracon-guard
# Group=dracon-guard

# Resource limits
MemoryMax=100M
CPUQuota=10%

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable dracon-system-guard
sudo systemctl start dracon-system-guard
```

## Usage

### Commands

```bash
# Show system status
dracon-system status

# Run diagnostics
dracon-system doctor

# Analyze storage hotspots
dracon-system storage ~/Dev

# Clean up build artifacts (dry-run)
dracon-system storage ~/Dev --cleanup

# Actually clean up
dracon-system storage ~/Dev --cleanup --apply

# Run single guard check
dracon-system guard once

# Run as daemon (continuous monitoring)
dracon-system guard daemon
```

### Systemd Service Management

```bash
# Enable at login
systemctl --user enable dracon-system-guard

# Start now
systemctl --user start dracon-system-guard

# Check status
systemctl --user status dracon-system-guard

# View logs
journalctl --user -u dracon-system-guard -f
```

## Configuration

Create `~/dracon/utilities/system/dracon-system.toml`:

```toml
[guard]
# Enable the guard daemon
enabled = true

# Check interval in seconds
interval_secs = 30

# Disk thresholds (percent)
disk_early_warn_percent = 70
disk_warn_percent = 80
disk_action_percent = 90
disk_critical_percent = 95

# Automatic Rust target cleanup
auto_cleanup_rust = true
cleanup_min_size_mb = 256
rust_search_roots = "~/Dev"  # Default; add more paths as needed
protect_recent_minutes = 30

# Trend prediction
track_trends = true
trend_warn_hours = 24

# Inode monitoring
monitor_inodes = true
inode_warn_percent = 85

# Zombie process detection
monitor_zombies = true
zombie_threshold = 20

# Large log file detection
monitor_logs = true
log_size_mb = 100
log_dirs = "/var/log,~/logs"

# Notifications
notify = true
notify_command = "notify-send"
notify_cooldown_secs = 300

# Sync freeze (for use with dracon-sync)
freeze_sync_at_action = true
unfreeze_below_percent = 88
```

## How It Works

### Threshold Actions

| State | Threshold | Actions |
|-------|-----------|---------|
| early-warn | 70% | Notification only |
| warn | 80% | Notification, state change alert |
| action | 90% | Freeze sync, auto-cleanup Rust targets |
| critical | 95% | All above, more aggressive cleanup |

### Cleanup Logic

When disk hits action level (90%):

1. Scan configured directories for Rust `target/` dirs
2. Detect active `cargo`/`rustc` processes
3. Protect target dirs:
   - In active build working directories
   - Modified within `protect_recent_minutes`
4. Delete unprotected target dirs ≥ `cleanup_min_size_mb`
5. Send notification with cleanup summary

### Trend Prediction

The guard tracks disk usage over time and uses linear regression to predict when the disk will fill. If the predicted time is within `trend_warn_hours`, it sends an early warning.

## Binary Size

The release binary is approximately 2.9MB, making it suitable for:
- Embedded systems
- Containers
- Minimal server installs

## License

MIT