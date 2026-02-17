# Clipboard Manager

A powerful clipboard history manager built with GPUI (the UI framework powering Zed editor).

## Features

- ✅ **Clipboard History Tracking** - Automatically captures all clipboard changes
- ✅ **Multi-Format Support** - Text, Rich Text (RTF), HTML, and Images
- ✅ **Virtual List** - High-performance rendering for large clipboard histories
- ✅ **Search & Filter** - Quickly find clipboard entries
- ✅ **Embedded Database** - Uses redb for fast, reliable storage
- ✅ **Event-Based Monitoring** - Efficient clipboard watching with clipboard-rs
- 🚧 **System Tray Icon** - Quick access from menu bar (coming soon)
- 🚧 **Global Shortcuts** - Show/hide with customizable hotkeys (coming soon)
- 🚧 **Settings UI** - Configure theme, language, and history limits (coming soon)

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
