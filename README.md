# Clipboard Manager

A powerful clipboard history manager built with GPUI (the UI framework powering Zed editor).

## Features

- ✅ **Clipboard History Tracking** - Automatically captures all clipboard changes
- ✅ **Multi-Format Support** - Text, Rich Text (RTF), HTML, and Images
- ✅ **Virtual List** - High-performance rendering for large clipboard histories
- ✅ **Search & Filter** - Quickly find clipboard entries
- ✅ **Embedded Database** - Uses redb for fast, reliable storage
- ✅ **Event-Based Monitoring** - Efficient clipboard watching with clipboard-rs
- ✅ **System Tray Icon** - Quick access from menu bar
- ✅ **Global Shortcuts** - Show/hide with customizable hotkeys
- ✅ **Settings UI** - Configure theme, language, and history limits

## Requirements

- macOS (initial version)
- Rust 1.70+

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run
```

## Release Automation (GitHub)

This repository includes a GitHub Actions workflow at `.github/workflows/release.yml` to build and publish release artifacts automatically.

### Trigger

- Push a version tag matching `v*` (for example: `v0.1.0`)
- Or run the workflow manually from the Actions tab (`workflow_dispatch`)

### How to publish a release

```bash
git tag v0.1.0
git push origin v0.1.0
```

### Generated artifacts

- `clipboard-manager-<tag>-macos.tar.gz`
- `clipboard-manager-<tag>-macos.tar.gz.sha256`

The workflow also creates a GitHub Release automatically and uploads these files.

## Usage

1. Launch the application
2. Copy text, images, or rich content
3. View your clipboard history in the main window
4. Click any item to copy it back to clipboard
5. Use the search bar to filter entries

## Configuration

Settings are stored in `~/Library/Application Support/clipboard-manager/settings.json`:

```json
{
  "theme": "System",
  "language": "English",
  "max_history_count": 100,
  "global_shortcut": "Cmd+Shift+V"
}
```

## Architecture

- **Database Layer** (`src/db/`) - redb-based storage for clipboard entries
- **Clipboard Monitor** (`src/clipboard/`) - Event-based clipboard watching
- **UI Components** (`src/ui/`) - GPUI components including virtual list
- **Settings** (`src/settings/`) - Configuration management
- **Tray** (`src/tray/`) - System tray integration (WIP)
- **Shortcuts** (`src/shortcuts/`) - Global hotkey handling (WIP)

## License

MIT

## Acknowledgments

- Built with [GPUI](https://github.com/zed-industries/zed)
- UI components from [gpui-component](https://github.com/longbridge/gpui-component)
- Clipboard handling with [clipboard-rs](https://github.com/ChurchTao/clipboard-rs)
