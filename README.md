# 🐉 iscsimon – The iSCSI Whisperer

*A delightfully interactive TUI for peering into the mysterious world of iSCSI connections.*

## What is this sorcery? ✨

`iscsimon` is a terminal user interface (TUI) that lets you commune with your Linux iSCSI target host. It transforms a maze of network connections into a beautiful, at-a-glance view of who's connecting to your precious block storage, where they're coming from, and how frantically they're copying data back and forth.

Think of it as a crystal ball for storage administrators – except it actually works and doesn't require a weird crystal.

## Features 🎯

- **Connection Voyeurism**: See all currently active iSCSI connections to your target host
- **Network Archaeology**: Discover the source of each connection (IP, port) – basically network forensics but friendlier
- **Target Identity Crisis**: Know exactly which iSCSI target is being accessed and which block device it's pretending to be
- **Speed Watching**: Monitor TX/RX rates in real-time and feel the data flow through your fingertips
- **Pretty Buttons**: A beautiful, keyboard-friendly TUI that makes monitoring feel less like a punishment

## Installation 🚀

```bash
cargo build --release
./target/release/iscsimon
```

## Usage 🎮

Run the tool and watch your iSCSI connections dance across your screen. Keyboard shortcuts make navigation a breeze:
- Arrow keys to navigate (probably)
- `q` to quit (the universal language of terminals)
- Magic happens when you press things

*Full command documentation coming soon, or just press `?` and cross your fingers.*

## Requirements 📋

- **OS**: A Linux system with iSCSI capabilities
- **Rust**: For building this monstrosity (edition 2024)
- **Dependencies**:
  - `ratatui` – Because TUIs should be pretty
  - `crossterm` – For cross-platform terminal wizardry
  - `chrono` – So time can flow correctly
  - `anyhow` – For when things inevitably go wrong

## The Vision 🎨

When complete, `iscsimon` will be the lovingly crafted monitoring tool you never knew you needed – a real-time window into your iSCSI fabric that actually sparks joy.

---

*Made with robots.
