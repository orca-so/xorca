# xORCA Staking Program

A Solana program for staking ORCA tokens with additional features and utilities.

## Quick Start with Docker

The easiest way to get started with xORCA is using Docker:

```bash
# Start development environment
docker compose --profile dev up

# Run tests
docker compose --profile test up

# Build all components
docker compose --profile build up
```

For more detailed Docker instructions, see [DOCKER.md](./DOCKER.md).

## Project Structure

- `solana-program/` - The main Solana program implementation
- `rust-client/` - Rust client library for interacting with the program
- `js-client/` - JavaScript/TypeScript client library
- `solana-program-test/` - Test suite for the Solana program

## Development

### Prerequisites

- Rust (latest stable)
- Node.js (LTS)
- Yarn
- Solana CLI tools

### Local Development

1. Install dependencies:

   ```bash
   yarn install
   cargo build
   ```

2. Build the Solana program:

   ```bash
   cargo build-sbf
   ```

3. Run tests:
   ```bash
   cargo test
   yarn workspace @orca-so/xorca test
   ```

## Documentation

- [Docker Setup](./DOCKER.md) - Complete Docker setup guide
- [API Documentation](./js-client/README.md) - JavaScript client documentation

## License

See [LICENSE](./LICENSE) for details.
