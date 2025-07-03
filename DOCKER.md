# Docker Setup for xORCA

This document explains how to use the Docker Compose setup for the xORCA project.

## Prerequisites

- Docker Desktop for Mac
- Docker Compose

## macOS Optimizations

This Docker setup is optimized for macOS with the following features:

- **Delegated volume mounting**: Uses `:delegated` flag for better performance on macOS
- **Platform specification**: Explicitly sets `platform: linux/amd64` for compatibility
- **Memory limits**: Configures appropriate memory limits for the Solana validator
- **Resource management**: Optimized for Docker Desktop on macOS

## Available Profiles

The Docker Compose configuration supports several profiles for different use cases:

### Development (`dev`)

Starts a local Solana test validator with the xORCA program deployed and builds the JS client.

```bash
docker compose --profile dev up
```

This will:

- Build the Solana program
- Start a local Solana test validator with the xORCA program deployed
- Start program logs
- Build the JS client

### Testing (`test`)

Runs all tests for both Rust and JavaScript components.

```bash
docker compose --profile test up
```

This will:

- Build the Solana program
- Run Rust tests
- Run JavaScript tests

### Building (`build`)

Builds all components for production.

```bash
docker compose --profile build up
```

This will:

- Build the Solana program
- Build the JS client
- Build the Rust client
- Generate IDL files

### Publishing (`publish`)

Publishes packages to npm and crates.io.

```bash
docker compose --profile publish up
```

**Note:** Requires `NPM_TOKEN` and `CARGO_TOKEN` environment variables to be set.

## Environment Variables

For publishing, you need to set the following environment variables:

- `NPM_TOKEN`: Your npm authentication token
- `CARGO_TOKEN`: Your crates.io authentication token

## Services

### Core Services

- **build-solana-program**: Builds the xORCA Solana program
- **solana-program-validator**: Runs a local Solana test validator
- **solana-program-logs**: Shows program logs
- **build-js**: Builds the JavaScript client
- **build-rust**: Builds the Rust client
- **build-idl**: Generates IDL files

### Test Services

- **test-rust**: Runs Rust tests
- **test-js**: Runs JavaScript tests

### Publish Services

- **publish-js**: Publishes the JS client to npm
- **publish-rust**: Publishes the Rust client to crates.io

## Ports

The Solana test validator exposes the following ports:

- `1024`: Gossip
- `1027`: TPU
- `8899`: RPC
- `8900`: WebSocket

## Volumes

- `xorca_cargo_home`: Cached Cargo dependencies

## Examples

### Start Development Environment

```bash
# Start all dev services
docker compose --profile dev up

# Start in detached mode
docker compose --profile dev up -d

# View logs
docker compose --profile dev logs -f
```

### Run Tests

```bash
# Run all tests
docker compose --profile test up

# Run tests and exit
docker compose --profile test up --abort-on-container-exit
```

### Build for Production

```bash
# Build all components
docker compose --profile build up
```

### Clean Up

```bash
# Stop all services
docker compose down

# Remove volumes
docker compose down -v

# Remove all containers and images
docker compose down --rmi all --volumes --remove-orphans
```

## macOS-Specific Tips

### Performance Optimization

- Ensure Docker Desktop has sufficient memory allocated (recommended: 8GB+)
- Use the delegated volume mounting for better I/O performance
- Consider using Docker Desktop's "Use the new Virtualization framework" option for better performance on Apple Silicon Macs

### Troubleshooting

- If you encounter permission issues, ensure Docker Desktop has access to the project directory
- For Apple Silicon Macs, the setup uses `linux/amd64` platform for maximum compatibility
- If builds are slow, consider increasing Docker Desktop's CPU and memory allocation

## Troubleshooting

### Common Issues

1. **Port conflicts**: Make sure ports 8899, 8900, 1024, and 1027 are not in use
2. **Permission issues**: Ensure Docker Desktop has proper permissions to access the project directory
3. **Build failures**: Check that all dependencies are properly installed and the project structure is correct
4. **Memory issues**: Increase Docker Desktop memory allocation if the Solana validator fails to start

### Debugging

- Use `docker compose logs <service-name>` to view logs for a specific service
- Use `docker compose ps` to check service status
- Use `docker compose exec <service-name> <command>` to run commands in a running container
