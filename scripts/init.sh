#!/bin/bash
# Rust Base Template Initialization Script
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

# Function to display usage
usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Initialize a new Rust project from the base template."
    echo ""
    echo "The template uses:"
    echo "  - Rust 2024 edition"
    echo "  - MIT OR Apache-2.0 dual license"
    echo "  - Copyright: Erwan Patrick Legrand"
    echo ""
    echo "OPTIONS:"
    echo "  --name NAME              Project name (required)"
    echo "  --description DESC       Project description (required)"
    echo "  --repository URL         Repository URL (optional)"
    echo "  --help                   Show this help message"
    echo ""
    echo "Example:"
    echo "  $0 --name my-project --description \"My awesome project\""
}

# Parse command line arguments
PROJECT_NAME=""
PROJECT_DESCRIPTION=""
REPOSITORY_URL=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --name)
            PROJECT_NAME="$2"
            shift 2
            ;;
        --description)
            PROJECT_DESCRIPTION="$2"
            shift 2
            ;;
        --repository)
            REPOSITORY_URL="$2"
            shift 2
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Validate required parameters
if [[ -z "$PROJECT_NAME" ]]; then
    log_error "Project name is required"
    usage
    exit 1
fi

if [[ -z "$PROJECT_DESCRIPTION" ]]; then
    log_error "Project description is required"
    usage
    exit 1
fi

# Get current year
CURRENT_YEAR=$(date +%Y)

# Create snake_case version of project name
PROJECT_NAME_SNAKE=$(echo "$PROJECT_NAME" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]/_/g')

# Fixed values
LICENSE="MIT OR Apache-2.0"
LICENSE_URL="https://opensource.org/licenses/MIT OR https://www.apache.org/licenses/LICENSE-2.0"

# Function to replace template variables in a file
replace_vars() {
    local file="$1"
    local temp_file=$(mktemp)

    # Use sed with a different delimiter to avoid issues with URLs
    sed \
        -e "s|{{project_name}}|$PROJECT_NAME|g" \
        -e "s|{{project_name_snake}}|$PROJECT_NAME_SNAKE|g" \
        -e "s|{{project_description}}|$PROJECT_DESCRIPTION|g" \
        -e "s|{{repository_url}}|${REPOSITORY_URL:-https://github.com/erwanlegrand/$PROJECT_NAME}|g" \
        -e "s|{{license_url}}|$LICENSE_URL|g" \
        -e "s|{{current_year}}|$CURRENT_YEAR|g" \
        "$file" > "$temp_file"

    # Replace original file
    mv "$temp_file" "$file"
}

# Function to copy and process template files
process_template() {
    local template_dir="$1"
    local target_dir="$2"

    log_info "Processing templates from $template_dir to $target_dir"

    # Find all template files
    find "$template_dir" -name "*.template" -type f | while read -r template_file; do
        # Calculate relative path
        relative_path="${template_file#$template_dir/}"
        target_file="$target_dir/${relative_path%.template}"

        # Create target directory if it doesn't exist
        mkdir -p "$(dirname "$target_file")"

        # Copy and process template
        cp "$template_file" "$target_file"
        replace_vars "$target_file"

        log_info "Created: $target_file"
    done
}

# Main initialization process
main() {
    log_info "Initializing Rust project: $PROJECT_NAME"
    log_info "Description: $PROJECT_DESCRIPTION"
    log_info "License: $LICENSE"
    log_info "Edition: 2024"

    # Check if we're in the template directory
    if [[ ! -d "templates/project" ]]; then
        log_error "Please run this script from the rust-base-template directory"
        exit 1
    fi

    # Check if target directory already exists
    if [[ -d "$PROJECT_NAME" ]]; then
        log_warn "Directory '$PROJECT_NAME' already exists"
        read -p "Do you want to overwrite it? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_info "Initialization cancelled"
            exit 0
        fi
        rm -rf "$PROJECT_NAME"
    fi

    # Create project directory
    mkdir -p "$PROJECT_NAME"
    log_info "Created project directory: $PROJECT_NAME"

    # Process templates
    process_template "templates/project" "$PROJECT_NAME"

    # Copy license files (dual licensing: MIT OR Apache-2.0)
    cp "templates/project/LICENSE-MIT.template" "$PROJECT_NAME/LICENSE-MIT"
    cp "templates/project/LICENSE-APACHE.template" "$PROJECT_NAME/LICENSE-APACHE"
    replace_vars "$PROJECT_NAME/LICENSE-MIT"
    replace_vars "$PROJECT_NAME/LICENSE-APACHE"

    # Initialize git repository
    cd "$PROJECT_NAME"
    if command -v git >/dev/null 2>&1; then
        log_info "Initializing git repository"
        git init
        git add .
        git commit -m "Initial commit: $PROJECT_NAME"
    else
        log_warn "Git not found, skipping repository initialization"
    fi

    # Run initial build to verify everything works
    log_info "Running initial build check"
    if command -v cargo >/dev/null 2>&1; then
        cargo check
        log_success "Project initialized successfully!"
        log_info ""
        log_info "Next steps:"
        log_info "  cd $PROJECT_NAME"
        log_info "  cargo build"
        log_info "  cargo test"
        log_info ""
        log_info "For development with full tooling, consider using:"
        log_info "  rust-full-template (includes dev container, pre-commit, xtask)"
    else
        log_warn "Cargo not found, skipping build check"
        log_success "Project files created successfully!"
    fi
}

# Run main function
main "$@"