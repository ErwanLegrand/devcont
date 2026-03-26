#!/usr/bin/env bash
set -euo pipefail

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info()    { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn()    { echo -e "${YELLOW}[WARN]${NC} $1"; }

log_info "Setting up devcont development environment..."

# Install/verify cargo tools
for tool in cargo-llvm-cov cargo-deny; do
    if ! cargo install --list | grep -q "^${tool} "; then
        log_info "Installing ${tool}..."
        cargo install "${tool}"
    fi
done
log_success "Cargo tools ready"

# Configure git defaults
git config --global init.defaultBranch main 2>/dev/null || true
git config --global pull.rebase false 2>/dev/null || true
git config --global core.autocrlf false 2>/dev/null || true

[[ -z "$(git config --global user.name 2>/dev/null)" ]] && \
    log_warn "git user.name not set — run: git config --global user.name 'Your Name'"
[[ -z "$(git config --global user.email 2>/dev/null)" ]] && \
    log_warn "git user.email not set — run: git config --global user.email 'you@example.com'"

log_success "Git configured"

# Quick build check
if [[ -f "Cargo.toml" ]]; then
    log_info "Running cargo check..."
    cargo check --quiet && log_success "Project builds successfully"
fi

# Check Docker socket accessibility (DooD)
if [[ -S /var/run/docker.sock ]] && docker version &>/dev/null; then
    log_success "Docker socket accessible (DooD ready)"
else
    log_warn "Docker socket not accessible — ensure /var/run/docker.sock is mounted from the host"
fi

echo ""
log_success "devcont development environment is ready!"
echo ""
echo "  cargo build          # Build the project"
echo "  cargo test           # Run tests"
echo "  cargo clippy         # Lint"
echo "  cargo llvm-cov       # Coverage"
