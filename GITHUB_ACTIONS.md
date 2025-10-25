# GitHub Actions - Automatic Docker Builds

This repository includes a GitHub Actions workflow that automatically builds and publishes Docker images to GitHub Container Registry (ghcr.io).

## What it does

The workflow automatically:
- Builds the Docker image for both **amd64** and **arm64** architectures
- Pushes images to GitHub Container Registry
- Tags images appropriately (latest, version tags, branch names)
- Uses build cache to speed up builds

## When it runs

The workflow triggers on:
- **Push to master branch** - Builds and pushes with `latest` tag
- **New version tags** (e.g., `v1.0.0`) - Builds and pushes with version tags
- **Pull requests** - Builds only (doesn't push)
- **Manual trigger** - Via GitHub Actions UI

## Using the published images

Once pushed, you can pull and run the image:

```bash
# Pull the latest image
docker pull ghcr.io/bifr0est/iso2god-rs-web:latest

# Run it
docker run -d --name iso2god-web \
  -p 8000:8000 \
  -v /path/to/isos:/data/input:ro \
  -v /path/to/output:/data/output \
  ghcr.io/bifr0est/iso2god-rs-web:latest
```

## Available tags

- `latest` - Latest build from master branch
- `master` - Latest build from master branch
- `v1.0.0` - Specific version (when you create a git tag)
- `1.0` - Major.minor version
- `1` - Major version only

## Supported architectures

The images are built for:
- **linux/amd64** - Standard x86_64 systems, Unraid, most servers
- **linux/arm64** - ARM64 systems like Raspberry Pi 4, Apple Silicon Macs

## Creating a new release

To create a versioned release:

```bash
# Tag the current commit
git tag -a v1.0.0 -m "Release v1.0.0"

# Push the tag
git push fork v1.0.0
```

This will trigger the workflow to build and push images tagged as:
- `ghcr.io/bifr0est/iso2god-rs-web:v1.0.0`
- `ghcr.io/bifr0est/iso2god-rs-web:1.0`
- `ghcr.io/bifr0est/iso2god-rs-web:1`
- `ghcr.io/bifr0est/iso2god-rs-web:latest`

## Viewing published packages

You can view published images at:
https://github.com/bifr0est?tab=packages

## Build status

Check the build status in the **Actions** tab of your GitHub repository:
https://github.com/bifr0est/iso2god-rs-web/actions

## Making images public

By default, GitHub Container Registry images are private. To make them public:

1. Go to https://github.com/bifr0est?tab=packages
2. Click on your `iso2god-rs-web` package
3. Click "Package settings" (right side)
4. Scroll down to "Danger Zone"
5. Click "Change visibility" → "Public"

## Updating your docker-compose.yml

You can now use the GitHub Container Registry image in your docker-compose.yml:

```yaml
version: '3.8'

services:
  iso2god-web:
    image: ghcr.io/bifr0est/iso2god-rs-web:latest
    container_name: iso2god-web
    ports:
      - "8000:8000"
    volumes:
      - /mnt/user/isos:/data/input:ro
      - /mnt/user/converted:/data/output
    restart: unless-stopped
```

This way you don't need to build the image locally - just pull and run!

## Troubleshooting

### Authentication required
If you get "authentication required" when pulling:
1. The package might be private (see "Making images public" above)
2. Or authenticate with: `echo $GITHUB_TOKEN | docker login ghcr.io -u USERNAME --password-stdin`

### Build fails
Check the Actions tab for build logs. Common issues:
- Rust compilation errors
- Dockerfile syntax errors
- Missing dependencies
