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
  - `src/generated/` - Auto-generated client code from IDL
  - `src/math/` - Math utility functions (compiled to WASM)
- `js-client/` - JavaScript/TypeScript client library
  - `src/generated/` - Auto-generated client code from IDL
  - `src/generated/wasm/` - Generated WASM bindings and TypeScript wrappers
- `solana-program-test/` - Test suite for the Solana program
- `scripts/` - Build scripts and utilities
- `codama.js` - Code generation script using Codama framework

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
   - Generate TypeScript and Rust client code using Codama
   - Build the TypeScript SDK
   - Build the Rust SDK with WASM features
   - Format code with Prettier and cargo fmt

3. Or build individual components:

   ```bash
   # Build contract and generate IDL
   yarn build:contract

   # Generate client code from IDL using Codama
   yarn generate

   # Build TypeScript SDK only
   yarn build:ts

   # Build Rust SDK only (with WASM features)
   yarn build:rs
   ```

4. Run tests:
   ```bash
   cargo test
   yarn workspace @orca-so/xorca test
   ```

### Code Generation

The project uses [Codama](https://github.com/codama-ai/codama) for generating type-safe client code from the Solana program IDL. The generation process:

1. Builds the Solana program and generates IDL using shank
2. Uses Codama to generate TypeScript and Rust client code
3. Includes account discriminators and padding fields
4. Generates WASM bindings for Rust math functions

The generated code is placed in:

- `js-client/src/generated/` - TypeScript client code
- `rust-client/src/generated/` - Rust client code

### Available Scripts

- `yarn build` - Build everything locally (contract, generate code, build SDKs, format)
- `yarn build:docker` - Build everything using Docker (no local dependencies needed)
- `yarn build:contract` - Build the Solana program and generate IDL
- `yarn build:ts` - Build the TypeScript SDK only
- `yarn build:rs` - Build the Rust SDK with WASM features
- `yarn generate` - Generate client code from the Solana program IDL using Codama
- `yarn clean` - Clean generated artifacts (generated code, IDL, WASM packages)
- `yarn fmt` - Format code with Prettier and cargo fmt

### Workspaces

The project uses Yarn workspaces for managing multiple packages:

- `js-client` - The main JavaScript/TypeScript client library (`@orca-so/xorca`)
- `test-server` - Test server for development (referenced in workspaces)

## Documentation

- [Docker Setup](./DOCKER.md) - Complete Docker setup guide
- [API Documentation](./js-client/README.md) - JavaScript client documentation

## License

See [LICENSE](./LICENSE) for details.
