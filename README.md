# iso2god-rs-web

A web interface for converting Xbox 360 and original Xbox ISOs into an Xbox 360 compatible Games-On-Demand file format, with built-in FTP transfer support.

This is a fork of [iliazeus/iso2god-rs](https://github.com/iliazeus/iso2god-rs), which is an optimized rewrite of [eliecharra/iso2god-cli](https://github.com/eliecharra/iso2god-cli), with additional web interface and FTP features.

## Features

- **Web Interface** - Modern, user-friendly web UI for ISO conversion
- **Automatic FTP Transfer** - Direct transfer to modded Xbox 360 consoles after conversion
- **Multi-threaded Processing** - Automatic CPU detection for optimal performance
- **Docker Support** - Easy deployment on NAS systems (Unraid, etc.)
- **Conversion History** - Track your converted games
- **Progress Tracking** - Real-time conversion progress with animated status

## Quick Start with Docker

### Using Docker Compose (Recommended)

```bash
docker-compose up -d
```

Then open http://localhost:8000 in your browser.

### Using Docker CLI

```bash
docker run -d \
  --name iso2god-web \
  -p 8000:8000 \
  -v /path/to/your/isos:/data/input:ro \
  -v /path/to/converted/games:/data/output \
  ghcr.io/bifr0est/iso2god-rs-web:latest
```

Replace `/path/to/your/isos` and `/path/to/converted/games` with your actual paths.

### Docker Hub Images

Multi-architecture images are available:
- `ghcr.io/bifr0est/iso2god-rs-web:latest` - Latest stable version
- `ghcr.io/bifr0est/iso2god-rs-web:master` - Latest master branch build

Supported platforms:
- `linux/amd64` (x86_64)
- `linux/arm64` (ARM64)

## Usage

1. **Access the Web Interface**
   - Open http://localhost:8000 in your browser

2. **Convert an ISO**
   - Select an ISO file from the dropdown
   - Choose thread count (or use Auto for optimal performance)
   - Optionally enable "Auto-Transfer to Xbox 360"
   - Click "Convert to GOD" (or "Convert & Transfer")

3. **FTP Transfer (Optional)**
   - Enable "Auto-Transfer to Xbox 360" checkbox
   - Enter your Xbox 360 FTP credentials
   - Converted games will automatically transfer after conversion

4. **View Converted Games**
   - Check the "Converted Games" section
   - Transfer any game to your Xbox 360 via FTP

## Configuration

### Environment Variables

- `ROCKET_PORT` - Server port (default: 8000)
- `ROCKET_ADDRESS` - Server address (default: 0.0.0.0)

### Volume Mounts

- `/data/input` - Mount your ISO files here (read-only recommended)
- `/data/output` - Converted GOD files will be written here

## Command Line Interface

The original CLI tool is still available:

```bash
Usage: iso2god [OPTIONS] <SOURCE_ISO> <DEST_DIR>

Arguments:
  <SOURCE_ISO>  ISO file to convert
  <DEST_DIR>    A folder to write resulting GOD files to

Options:
      --dry-run             Do not convert anything, just print the title info
      --game-title <TITLE>  Set game title
      --trim                Trim off unused space from the ISO image
  -j, --num-threads <N>     Number of worker threads to use
  -h, --help                Print help
  -V, --version             Print version
```

## Building from Source

### Prerequisites

- Rust 1.70 or later
- Docker (for containerized builds)

### Build Web Interface

```bash
cargo build --release --bin iso2god-web
./target/release/iso2god-web
```

### Build Docker Image

```bash
docker build -t iso2god-web .
```

## Development

### Project Structure

- `src/bin/iso2god-web.rs` - Web server implementation (Rocket framework)
- `templates/` - HTML templates (Tera templating)
- `public/` - Static assets (CSS, JavaScript)
- `.github/workflows/` - CI/CD workflows

### Automated Updates

This project uses:
- **Dependabot** - Automatic dependency updates
- **GitHub Actions** - Automated Docker builds and SonarQube analysis

## License

MIT License - See LICENSE file for details

## Credits

- Original project: [iliazeus/iso2god-rs](https://github.com/iliazeus/iso2god-rs)
- Based on: [eliecharra/iso2god-cli](https://github.com/eliecharra/iso2god-cli)
- Web interface and FTP support: [@bifr0est](https://github.com/bifr0est)
