FROM buildpack-deps:bullseye

# Install Node.js (using NodeSource)
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - && \
    apt-get update && apt-get install -y nodejs && \
    corepack enable && corepack prepare yarn@stable --activate

# Install Rust and tools
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.88.0 && \
    export PATH="/root/.cargo/bin:$PATH" && \
    cargo install wasm-pack && \
    rustup component add rustfmt clippy && \
    cargo install shank-cli

ENV PATH="/root/.cargo/bin:$PATH"

# Set working directory
WORKDIR /usr/src/xorca

# Copy package files for better caching
COPY package.json yarn.lock ./
COPY js-client/package.json ./js-client/
COPY rust-client/Cargo.toml ./rust-client/

# Install dependencies
RUN yarn install --frozen-lockfile

# Copy the rest of the source code
COPY . .

# Set environment variables
ENV CARGO_HOME=/root/.cargo

# Default command
CMD ["bash"] 