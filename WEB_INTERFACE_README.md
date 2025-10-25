# iso2god-rs Web Interface

A web-based interface for converting Xbox 360 ISO files to Games on Demand (GOD) format, designed to run in Docker on your NAS (especially Unraid).

## Features

- **No Upload Required**: Browse and select ISO files directly from your mounted NAS directory
- **Optional Upload**: Still supports uploading ISOs if needed
- **Real-time Status**: See conversion progress and detailed results
- **Game Detection**: Automatically identifies games from the built-in database
- **Configurable Options**: Control trim mode, thread count, and output location
- **Dry Run Mode**: Preview game info without converting

## Quick Start for Unraid

1. Edit `docker-compose.yml` and update the volume paths:
   ```yaml
   volumes:
     - /mnt/user/your-isos:/data/input:ro
     - /mnt/user/your-output:/data/output
   ```

2. Build and run:
   ```bash
   docker-compose up -d
   ```

3. Access at: `http://your-unraid-ip:8000`

## How It Works

The web interface provides a simple form where you can:

1. **Select an ISO** from a dropdown (populated from `/data/input`)
2. **Configure conversion settings**:
   - Destination directory (defaults to `/data/output`)
   - Game title override (optional)
   - Trim mode (from-end recommended to save space)
   - Thread count (1 for HDDs, 4-8 for SSDs)
   - Dry run option
3. **Click Convert** and wait for the process to complete
4. **View results** with detailed title information

## Architecture

- **Backend**: Rust with Rocket web framework
- **Frontend**: Pure HTML/CSS/JavaScript (no frameworks)
- **Container**: Multi-stage Docker build for minimal image size
- **Storage**: Direct file system access (no database needed)

## File Structure

```
.
├── src/bin/iso2god-web.rs    # Web server and conversion logic
├── templates/
│   └── index.html.tera       # Main web interface template
├── public/
│   ├── style.css             # Styling
│   └── script.js             # Frontend JavaScript
├── Dockerfile                # Multi-stage build
├── docker-compose.yml        # Easy deployment config
└── DOCKER_DEPLOYMENT.md      # Full deployment guide
```

## API Endpoints

- `GET /` - Main web interface
- `GET /list-isos` - Returns JSON list of available ISO files
- `POST /convert` - Performs the conversion (accepts multipart form data)
- `GET /public/*` - Static assets (CSS, JS)

## Configuration

All configuration is done via volume mounts in `docker-compose.yml`:

```yaml
volumes:
  # Your ISO files (read-only for safety)
  - /path/to/isos:/data/input:ro
  # Where converted GOD files go
  - /path/to/output:/data/output
```

Environment variables (optional):
```yaml
environment:
  - ROCKET_ADDRESS=0.0.0.0  # Listen address
  - ROCKET_PORT=8000        # Port number
```

## Performance Notes

- Conversions are CPU intensive
- Average conversion takes 5-15 minutes depending on ISO size
- Thread count of 1 is safe for HDDs
- SSDs can handle 4-8 threads for faster conversion
- Multiple simultaneous conversions will queue up

## Troubleshooting

**No ISOs appearing in dropdown:**
- Check that your volume mount is correct in docker-compose.yml
- Verify ISO files have .iso extension (case insensitive)
- Check container logs: `docker-compose logs -f`

**Permission errors:**
- Ensure output directory is writable: `chmod 777 /path/to/output`
- Check container user permissions

**Conversion fails:**
- Verify the ISO file is a valid Xbox 360 ISO
- Check available disk space in output directory
- Review error message for specific issues

## Development

To build and test locally:

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build
cargo build --release --bin iso2god-web

# Run
cargo run --bin iso2god-web
```

## Credits

Web interface by various contributors
Based on [iso2god-rs](https://github.com/iliazeus/iso2god-rs) by [iliazeus](https://github.com/iliazeus)

## License

Same as the original iso2god-rs project
