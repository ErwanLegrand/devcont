# Rust Development Container
FROM mcr.microsoft.com/devcontainers/rust:latest

# Install additional development tools
RUN apt-get update && apt-get install -y \
    # Build tools
    build-essential \
    cmake \
    pkg-config \
    # Database clients
    postgresql-client \
    redis-tools \
    mongodb-clients \
    # Network tools
    curl \
    wget \
    netcat \
    dnsutils \
    # Development utilities
    git \
    vim \
    htop \
    tree \
    jq \
    # Python (for scripts and tools)
    python3 \
    python3-pip \
    python3-venv \
    # Additional libraries
    libssl-dev \
    libpq-dev \
    libsasl2-dev \
    libldap2-dev \
    && rm -rf /var/lib/apt/lists/*

# Install cargo tools for enhanced development
RUN cargo install \
    # Development tools
    cargo-watch \
    cargo-expand \
    cargo-edit \
    cargo-outdated \
    # Testing and quality
    cargo-llvm-cov \
    cargo-nextest \
    # Documentation
    cargo-doc \
    # Utilities
    cargo-generate \
    cargo-tree \
    # Performance
    cargo-flamegraph \
    # Cross-compilation (optional)
    cross \
    && rm -rf /usr/local/cargo/registry

# Install additional tools that might not be available via cargo
RUN if command -v pip3 >/dev/null 2>&1; then \
    pip3 install --user \
        pre-commit \
        black \
        isort \
        mypy \
        ruff \
        && rm -rf ~/.cache/pip; \
    fi

# Install Node.js tools for web development (if needed)
RUN if command -v npm >/dev/null 2>&1; then \
    npm install -g \
        yarn \
        typescript \
        @types/node \
        && npm cache clean --force; \
    fi

# Set up the workspace
WORKDIR /workspace

# Create common directories
RUN mkdir -p \
    /workspace/target \
    /workspace/.vscode \
    /workspace/.devcontainer \
    /tmp/cargo-cache \
    /tmp/rustup-cache

# Configure cargo to use the cache directories
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_TARGET_DIR=/workspace/target
ENV RUST_BACKTRACE=1
ENV RUST_LOG=info

# Set the default shell to bash
ENV SHELL=/bin/bash

# Add cargo bin to PATH (should already be there, but just in case)
ENV PATH="/usr/local/cargo/bin:${PATH}"

# Configure git (will be overridden by user preferences)
RUN git config --global init.defaultBranch main && \
    git config --global pull.rebase false && \
    git config --global push.default simple

# Create a non-root user for development (handled by dev container features)
# The rust feature should handle this

# Pre-warm cargo registry cache with common dependencies
RUN cargo search --limit 1 serde && \
    cargo search --limit 1 tokio && \
    cargo search --limit 1 axum && \
    cargo search --limit 1 clap

# Set the default command
CMD ["sleep", "infinity"]