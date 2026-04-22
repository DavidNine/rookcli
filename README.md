# Rook-Ceph TUI Management Utility

A robust, terminal-based management tool for Rook-Ceph clusters, built with Rust and `ratatui`. This utility provides real-time monitoring and surgical management capabilities directly from your CLI.

## Key Features

- **🚀 High Performance:** Built on an asynchronous architecture with background polling, ensuring the UI remains fluid and responsive even during large Kubernetes API requests.
- **📊 Real-time Monitoring:** 
  - **Clusters:** View Ceph health status and overall pod readiness at a glance.
  - **Pools:** Monitor CephBlockPool status and configuration.
  - **Pods:** Comprehensive pod list in the `rook-ceph` namespace with Ready ratios, Status, Restart counts, and Node locations.
- **🛠️ Management Actions:**
  - **Restart/Delete Pods:** Perform surgical pod removals for quick restarts.
  - **Delete Pools:** Manage Ceph storage resources with built-in safety confirmations.
- **🔍 Deep Diagnostics:**
  - **Describe View:** View the full Kubernetes YAML specification and recent lifecycle events for any pod.
  - **Log Viewer:** Instant access to pod logs with automatic container detection (defaults to the first container if multiple are present).
- **⌨️ Intuitive Navigation:** Supports arrow keys, Tab-based switching, Page Up/Down jumping, and automatic wrap-around navigation.

## Keyboard Shortcuts

### Global / Navigation
- `Tab` / `Right Arrow`: Next Tab
- `Shift+Tab` / `Left Arrow`: Previous Tab
- `Up` / `Down`: Navigate lists (wraps around at top/bottom)
- `PageUp` / `PageDown`: Jump up/down (10 items)
- `q` / `Ctrl+C`: Quit

### Resource Specific (Pods Tab)
- `r`: **Restart** selected Pod (Delete with confirmation)
- `x`: **Delete** selected Pod (Delete with confirmation)
- `d`: **Describe** selected Pod (Open detailed YAML + Events view)
- `l`: **Logs** (View recent logs from the selected pod)

### Resource Specific (Pools Tab)
- `d`: **Delete** selected Pool (Delete with confirmation)

## Prerequisites

- Access to a Kubernetes cluster via `kubectl` (configured in your environment).
- Rook-Ceph installed in the `rook-ceph` namespace.
- Rust toolchain (if building from source).

## Quick Start

1. Clone the repository:
   ```bash
   git clone git@github.com:DavidNine/rookcli.git
   cd rookcli
   ```

2. Run the utility:
   ```bash
   cargo run
   ```

## Design Philosophy

The tool is designed for speed and reliability. By separating the UI rendering from the Kubernetes API polling, it avoids the "freezing" commonly associated with CLI tools that perform blocking network calls. It prioritizes safety through modal confirmation dialogs for all destructive operations.
