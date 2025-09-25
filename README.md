# EPLAN eVIEW Extractor

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![Windows](https://img.shields.io/badge/platform-windows-blue.svg)](https://github.com/Alexander423/EPLAN_eView_Extractor/releases)

> A fast, reliable tool for extracting PLC variable tables from EPLAN eVIEW projects.

Stop manually copying PLC variables from eVIEW one by one. This tool automatically extracts entire variable tables in seconds, not hours.

## Why This Tool Exists

EPLAN eVIEW is great for viewing electrical projects, but extracting PLC data is painful:
- Manual copy-paste takes forever
- Easy to miss variables or make mistakes
- No way to get data in a usable format for other tools

This extractor solves that by automating the entire process with a clean, modern interface.

## Screenshots

*[Screenshots will be added showing the main interface, extraction process, and exported results]*

## Features

🚀 **Fast Extraction** - Process entire projects in under a minute
🎯 **Smart Detection** - Automatically finds and categorizes PLC variables
📊 **Multiple Formats** - Export to Excel, CSV, or JSON
🔐 **Secure Login** - Uses your existing Microsoft credentials
🎨 **Modern UI** - Clean interface with dark/light themes
⌨️ **Keyboard Shortcuts** - Work efficiently with hotkeys
💾 **Auto-Save** - Never lose your settings
🔍 **Search & Filter** - Find variables instantly

## Quick Start

### Prerequisites

- Windows 10/11
- Google Chrome (any recent version)
- EPLAN eVIEW access with valid credentials

### Download & Run

1. Download the latest release from [Releases](https://github.com/Alexander423/EPLAN_eView_Extractor/releases)
2. Double-click `eview_scraper.exe` - no installation required
3. Enter your eVIEW credentials and project number
4. Click Extract and wait for results

That's it. The tool handles ChromeDriver automatically.

## Building from Source

### Requirements

- [Rust](https://rustup.rs/) (latest stable)
- Git

### Build Steps

```bash
git clone https://github.com/Alexander423/EPLAN_eView_Extractor.git
cd EPLAN_eView_Extractor
cargo build --release
```

The executable will be in `target/release/eview_scraper.exe`.

## Usage Guide

### Basic Workflow

1. **Configure** - Enter your Microsoft email, password, and project number
2. **Extract** - Click Extract or press Ctrl+E to start
3. **Review** - Browse the extracted variables in the table
4. **Export** - Choose your format and save the results

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+E` | Start extraction |
| `Ctrl+S` | Save settings |
| `Ctrl+L` | Switch to Logs tab |
| `Ctrl+R` | Switch to Results tab |
| `F5` | Restart extraction |
| `Esc` | Cancel operation |

### Configuration

Settings are automatically saved to:
- `%APPDATA%\\eplan\\eview-scraper\\config.json`

The tool remembers your credentials (password is not stored in plain text) and preferences between sessions.

### Export Formats

**Excel (.xlsx)**
- Formatted tables with color coding
- Separate sheets for different variable types
- Perfect for documentation

**CSV**
- Simple, universal format
- Easy to import into other tools
- Lightweight and fast

**JSON**
- Structured data with full metadata
- Ideal for automation and scripting
- Machine-readable format

## Troubleshooting

### Common Issues

**"ChromeDriver connection failed"**
- The tool downloads ChromeDriver automatically
- If issues persist, try running as Administrator

**"Login failed"**
- Verify your Microsoft credentials
- Check your internet connection
- Make sure you have eVIEW access for the project

**"Project not found"**
- Double-check the project number format
- Ensure you have access to the specified project
- Try searching for the project manually in eVIEW first

### Debug Mode

Enable debug mode in settings to:
- Keep the browser window visible
- See detailed extraction logs
- Troubleshoot connection issues

## Technical Details

- **Language**: Rust for performance and reliability
- **GUI**: egui for cross-platform native interface
- **Web Automation**: thirtyfour (WebDriver) for browser control
- **Authentication**: Microsoft OAuth2 integration
- **Exports**: Native Excel/CSV/JSON generation

## Contributing

Found a bug or want a feature? [Open an issue](https://github.com/Alexander423/EPLAN_eView_Extractor/issues).

Want to contribute code? Check out [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Project Status

This tool is actively maintained and used in production environments. New features and improvements are added regularly based on user feedback.

## License

MIT License - see [LICENSE](LICENSE) for details.

---

*Made with ❤️ for electrical engineers who value their time*