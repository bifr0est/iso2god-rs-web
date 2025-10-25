# iso2god-rs Web Interface - Docker Deployment Guide

This guide explains how to deploy the iso2god-rs web interface on your NAS (including Unraid) using Docker.

## Prerequisites

- Docker and Docker Compose installed on your NAS
- Access to your NAS terminal/SSH
- Existing Xbox 360 ISO files on your NAS

## Quick Start for Unraid

1. Clone or download this repository to your Unraid appdata:
   ```bash
   cd /mnt/user/appdata
   git clone https://github.com/iliazeus/iso2god-rs.git
   cd iso2god-rs
   ```

2. Edit `docker-compose.yml` to point to your existing ISO location:
   ```bash
   nano docker-compose.yml
   ```

   Update the volume paths to match your setup. For example:
   ```yaml
   volumes:
     - /mnt/user/xbox360/isos:/data/input:ro
     - /mnt/user/xbox360/god:/data/output
   ```

3. Build and start the container:
   ```bash
   docker-compose up -d
   ```

4. Access the web interface at: `http://your-unraid-ip:8000`

## Configuration

### Volume Mounts

You MUST edit the docker-compose.yml to point to your actual directories:
- `/data/input` (read-only) - Mount your existing ISO directory here
- `/data/output` - Mount where you want converted GOD files saved

Example for Unraid:

```yaml
volumes:
  - /path/to/your/isos:/data/input:ro
  - /path/to/your/output:/data/output
```

### Port Configuration

The default port is 8000. To change it, modify the port mapping in `docker-compose.yml`:

```yaml
ports:
  - "8080:8000"  # External:Internal
```

## Usage

1. Open the web interface in your browser at `http://your-nas-ip:8000`
2. **Select an ISO file** from your mounted directory (or upload one if needed)
   - The dropdown will automatically show all .iso files in your `/data/input` directory
   - Shows file size for each ISO
3. Set the destination directory (default: `/data/output`)
4. Optionally customize:
   - **Game title** - Override the detected game name
   - **Trim mode** - "from-end" removes unused space (recommended)
   - **Number of threads** - 1 recommended for HDDs, 4-8 for SSDs
   - **Dry run** - Just shows title info without converting
5. Click "Convert"
6. Wait for the conversion to complete (may take 5-15 minutes depending on ISO size)
7. Find your converted GOD files in the output directory

### Web Interface Features

- **Browse server ISOs**: No need to upload - select from your existing ISOs
- **Upload support**: Can still upload ISOs if needed
- **Real-time status**: See conversion progress and results
- **Title detection**: Automatically identifies games from database

## Building from Source

If you want to build the image manually:

```bash
docker build -t iso2god-web .
docker run -d \
  -p 8000:8000 \
  -v /path/to/isos:/data/input:ro \
  -v /path/to/output:/data/output \
  --name iso2god-web \
  iso2god-web
```

## Troubleshooting

### Container won't start
Check logs:
```bash
docker-compose logs -f
```

### Permission errors
Make sure the output directory is writable:
```bash
chmod 777 output
```

### Cannot access web interface
- Verify the container is running: `docker ps`
- Check firewall settings on your NAS
- Ensure port 8000 is not already in use

## Stopping the Service

```bash
docker-compose down
```

## Updating

```bash
git pull
docker-compose down
docker-compose up -d --build
```

## Notes for Unraid Users

- **No uploads needed**: The web interface reads ISOs directly from your mounted share
- ISO files are mounted read-only for safety
- Converted GOD files appear immediately in your output share
- Conversions are CPU intensive; monitor your server load
- The default thread count is 1 to protect HDDs; increase to 4-8 if using SSDs
- You can run multiple conversions, but they'll queue up and may slow down your server

## Unraid Community Applications

To add via Unraid CA (if/when available):
1. Go to Apps tab
2. Search for "iso2god"
3. Configure your paths and install

Until then, use docker-compose as shown above.

## Credits

Based on [iso2god-rs](https://github.com/iliazeus/iso2god-rs) by iliazeus
