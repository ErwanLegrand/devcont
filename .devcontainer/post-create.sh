#!/bin/bash
# Post-create script for Rust Development Container
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_info "Setting up Rust development environment..."

# Check if we're in the correct directory
if [[ ! -f "Cargo.toml" ]] && [[ ! -d ".devcontainer" ]]; then
    log_warn "Not in a project directory. Skipping project-specific setup."
    exit 0
fi

# Install pre-commit hooks if configuration exists
install_pre_commit() {
    if [[ -f ".pre-commit-config.yaml" ]]; then
        log_info "Installing pre-commit hooks..."

        if command -v pre-commit >/dev/null 2>&1; then
            pre-commit install
            pre-commit install --hook-type commit-msg 2>/dev/null || true
            log_success "Pre-commit hooks installed"
        else
            log_warn "pre-commit not found. Install with: pip install pre-commit"
        fi
    fi
}

# Set up Rust toolchain
setup_rust() {
    log_info "Configuring Rust toolchain..."

    # Ensure we have the latest nightly for Rust 2024
    if ! rustc --version | grep -q "nightly"; then
        log_info "Switching to nightly toolchain for Rust 2024 support..."
        rustup default nightly
    fi

    # Install/update components
    rustup component add rustfmt clippy rust-analyzer llvm-tools-preview

    # Install additional targets if needed
    # rustup target add x86_64-unknown-linux-musl
    # rustup target add wasm32-unknown-unknown

    log_success "Rust toolchain configured"
}

# Install cargo tools
install_cargo_tools() {
    log_info "Installing additional cargo tools..."

    # Check and install tools that might be missing
    local tools_to_install=()

    if ! cargo install --list | grep -q "cargo-watch"; then
        tools_to_install+=("cargo-watch")
    fi

    if ! cargo install --list | grep -q "cargo-edit"; then
        tools_to_install+=("cargo-edit")
    fi

    if ! cargo install --list | grep -q "cargo-llvm-cov"; then
        tools_to_install+=("cargo-llvm-cov")
    fi

    if [[ ${#tools_to_install[@]} -gt 0 ]]; then
        log_info "Installing: ${tools_to_install[*]}"
        cargo install "${tools_to_install[@]}"
    else
        log_info "All cargo tools already installed"
    fi

    log_success "Cargo tools ready"
}

# Configure Git
configure_git() {
    log_info "Configuring Git..."

    # Set up git configuration if not already set
    if [[ -z "$(git config --global user.name)" ]]; then
        log_warn "Git user.name not set. Run: git config --global user.name 'Your Name'"
    fi

    if [[ -z "$(git config --global user.email)" ]]; then
        log_warn "Git user.email not set. Run: git config --global user.email 'your.email@example.com'"
    fi

    # Configure git for better development experience
    git config --global init.defaultBranch main
    git config --global pull.rebase false
    git config --global push.default simple
    git config --global core.autocrlf false

    log_success "Git configured"
}

# Set up VS Code workspace
setup_vscode() {
    log_info "Setting up VS Code workspace..."

    # Create .vscode directory if it doesn't exist
    mkdir -p .vscode

    # Create or update settings.json
    if [[ ! -f ".vscode/settings.json" ]]; then
        cat > .vscode/settings.json << 'EOF'
{
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.cargo.features": "all",
  "editor.formatOnSave": true,
  "editor.codeActionsOnSave": {
    "source.fixAll": "explicit",
    "source.organizeImports": "explicit"
  },
  "files.exclude": {
    "**/target": true,
    "**/.git": true
  }
}
EOF
        log_info "Created .vscode/settings.json"
    fi

    log_success "VS Code workspace configured"
}

# Check and build project
check_project() {
    if [[ -f "Cargo.toml" ]]; then
        log_info "Checking project..."

        # Run cargo check
        if cargo check; then
            log_success "Project builds successfully"
        else
            log_error "Project has build errors. Check the code."
            exit 1
        fi

        # Run tests if they exist
        if cargo test --lib --no-run 2>/dev/null; then
            log_info "Running tests..."
            if cargo test --lib --quiet; then
                log_success "Tests pass"
            else
                log_warn "Some tests failed. Check the test output."
            fi
        fi
    fi
}

# Display welcome message
welcome_message() {
    echo ""
    log_success "Rust development environment is ready!"
    echo ""
    echo "Quick start commands:"
    echo "  cargo build          # Build the project"
    echo "  cargo test           # Run tests"
    echo "  cargo check          # Quick compilation check"
    echo "  cargo clippy         # Run linter"
    echo "  cargo doc --open     # Generate and open documentation"
    echo ""
    echo "Development tools available:"
    echo "  cargo watch          # Watch for changes and rebuild"
    echo "  cargo expand         # Expand macros"
    echo "  cargo llvm-cov       # Code coverage"
    echo ""
    if [[ -f ".pre-commit-config.yaml" ]]; then
        echo "Pre-commit hooks are installed and will run on commit."
        echo "   To run manually: pre-commit run --all-files"
        echo ""
    fi
    echo "For help, see the README.md file."
}

# Main setup process
main() {
    install_pre_commit
    setup_rust
    install_cargo_tools
    configure_git
    setup_vscode
    check_project
    welcome_message
}

# Run main function
main "$@"