# xORCA Staking Program

A Solana program for staking ORCA tokens with additional features and utilities.

## Quick Start with Docker

The easiest way to get started with xORCA is using Docker:

```bash
# Start development environment
docker compose --profile dev up

# Run tests
docker compose --profile test up

# Build all components (generates code, builds SDK, etc.)
yarn build:docker
```

For more detailed Docker instructions, see [DOCKER.md](./DOCKER.md).

## Project Structure

- `solana-program/` - The main Solana program implementation
- `rust-client/` - Rust client library for interacting with the program
- `js-client/` - JavaScript/TypeScript client library
- `solana-program-test/` - Test suite for the Solana program

## Development

### Prerequisites

For local development, you'll need to install the following tools:

- **Rust** (latest stable) - [Install Rust](https://rustup.rs/)
- **Node.js** (LTS) - [Install Node.js](https://nodejs.org/)
- **Yarn** - [Install Yarn](https://yarnpkg.com/getting-started/install)
- **Solana CLI tools** - [Install Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools)
- **shank-cli** - Install with `cargo install shank-cli`

For Docker-based builds, you only need:

- **Docker** - [Install Docker](https://docs.docker.com/get-docker/)

### Local Development

1. Install dependencies:

   ```bash
   yarn install
   cargo build
   ```

2. Build everything (contract, generate code, build SDKs):

   ```bash
   yarn build
   ```

   This will:

   - Build the Solana program and generate IDL using shank
   - Generate TypeScript and Rust client code using codama
   - Build the TypeScript SDK
   - Build the Rust SDK

3. Or build individual components:

   ```bash
   # Build contract and generate code
   yarn build:contract

   # Generate client code from IDL
   yarn generate

   # Build TypeScript SDK only
   yarn build:ts

   # Build Rust SDK only
   yarn build:rs
   ```

4. Run tests:
   ```bash
   cargo test
   yarn workspace @orca-so/xorca test
   ```

### Available Scripts

- `yarn build` - Build everything locally (contract, generate code, build SDKs)
- `yarn build:docker` - Build everything using Docker (no local dependencies needed)
- `yarn build:contract` - Build the Solana program and generate IDL
- `yarn build:ts` - Build the TypeScript SDK only
- `yarn build:rs` - Build the Rust SDK only
- `yarn generate` - Generate client code from the Solana program IDL
- `yarn clean` - Clean generated artifacts
- `yarn fmt` - Format code with Prettier

## Documentation

- [Docker Setup](./DOCKER.md) - Complete Docker setup guide
- [API Documentation](./js-client/README.md) - JavaScript client documentation

## License

See [LICENSE](./LICENSE) for details.
