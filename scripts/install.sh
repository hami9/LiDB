#!/bin/sh
# LiDB / LiDashBoard official installer and uninstaller script.
# Repository: https://github.com/hami9/LiDB
# License: MIT
set -eu

REPO="hami9/LiDB"
BINARY_NAME="lidash"
INSTALL_TAG=""
TARGET_DIR=""
UNINSTALL=0
DRY_RUN=0

usage() {
    cat <<EOF
LiDashBoard (lidash) Installer

Usage:
  install.sh [options]

Options:
  -d, --dir DIR      Installation target directory (default: /usr/local/bin or ~/.local/bin)
  -t, --tag TAG      Specific release version tag to install (e.g. v0.0.6, default: latest)
  -u, --uninstall    Remove existing lidash installation from target directory
  -n, --dry-run      Show planned actions without modifying filesystem
  -h, --help         Display this help message

Examples:
  curl -fsSL https://raw.githubusercontent.com/hami9/LiDB/main/scripts/install.sh | sh
  sh install.sh --dir ~/.local/bin
  sh install.sh --uninstall
EOF
}

log() {
    printf "==> %s\n" "$*"
}

error() {
    printf "Error: %s\n" "$*" >&2
    exit 1
}

# Parse command line options
while [ $# -gt 0 ]; do
    case "$1" in
        -d|--dir)
            TARGET_DIR="$2"
            shift 2
            ;;
        -t|--tag)
            INSTALL_TAG="$2"
            shift 2
            ;;
        -u|--uninstall)
            UNINSTALL=1
            shift
            ;;
        -n|--dry-run)
            DRY_RUN=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            error "Unknown argument: $1. Run with --help for options."
            ;;
    esac
done

# Detect operating system
OS="$(uname -s)"
case "$OS" in
    Linux)
        ;;
    *)
        error "Unsupported operating system: $OS. LiDB binary releases are built for Linux."
        ;;
esac

# Detect CPU architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)
        ASSET_NAME="lidb-x86_64"
        ;;
    aarch64|arm64)
        ASSET_NAME="lidb-aarch64"
        ;;
    *)
        error "Unsupported CPU architecture: $ARCH. Available release binaries: x86_64, aarch64."
        ;;
esac

# Default target directory selection
if [ -z "$TARGET_DIR" ]; then
    if [ "$(id -u)" -eq 0 ]; then
        TARGET_DIR="/usr/local/bin"
    else
        TARGET_DIR="$HOME/.local/bin"
    fi
fi

DEST_PATH="$TARGET_DIR/$BINARY_NAME"

# Handle uninstall request
if [ "$UNINSTALL" -eq 1 ]; then
    log "Uninstalling $BINARY_NAME from $TARGET_DIR..."
    if [ ! -f "$DEST_PATH" ]; then
        log "Binary $DEST_PATH does not exist. Nothing to remove."
        exit 0
    fi
    if [ "$DRY_RUN" -eq 1 ]; then
        log "[dry-run] Would remove $DEST_PATH"
        exit 0
    fi
    rm -f "$DEST_PATH"
    log "Successfully removed $DEST_PATH."
    exit 0
fi

# Detect download tool (curl or wget)
if command -v curl >/dev/null 2>&1; then
    FETCH_CMD="curl -fsSL"
elif command -v wget >/dev/null 2>&1; then
    FETCH_CMD="wget -qO-"
else
    error "Neither curl nor wget was found in PATH. Please install curl or wget."
fi

# Detect sha256 checksum tool
if command -v sha256sum >/dev/null 2>&1; then
    SHA_CMD="sha256sum"
elif command -v shasum >/dev/null 2>&1; then
    SHA_CMD="shasum -a 256"
else
    error "sha256sum or shasum is required for secure release verification."
fi

# Resolve version tag
if [ -z "$INSTALL_TAG" ]; then
    log "Finding latest release tag..."
    LATEST_JSON=$($FETCH_CMD "https://api.github.com/repos/$REPO/releases" 2>/dev/null || true)
    if [ -n "$LATEST_JSON" ]; then
        INSTALL_TAG=$(printf "%s" "$LATEST_JSON" | grep -o '"tag_name": *"[^"]*"' | head -n 1 | cut -d '"' -f 4)
    fi
    if [ -z "$INSTALL_TAG" ]; then
        # Fallback to /releases/latest endpoint
        LATEST_JSON=$($FETCH_CMD "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null || true)
        if [ -n "$LATEST_JSON" ]; then
            INSTALL_TAG=$(printf "%s" "$LATEST_JSON" | grep -o '"tag_name": *"[^"]*"' | head -n 1 | cut -d '"' -f 4)
        fi
    fi
    if [ -z "$INSTALL_TAG" ]; then
        # Fallback to direct redirect header lookup
        INSTALL_TAG=$($FETCH_CMD -I "https://github.com/$REPO/releases/latest" 2>/dev/null | grep -i "location:" | sed -E 's|.*/tag/(.*)|\1|' | tr -d '\r\n')
    fi
    if [ -z "$INSTALL_TAG" ]; then
        error "Could not automatically determine the latest release tag. Specify with --tag <tag>."
    fi
fi

log "Target release: $INSTALL_TAG"
log "Target architecture: $ARCH ($ASSET_NAME)"
log "Destination: $DEST_PATH"

if [ "$DRY_RUN" -eq 1 ]; then
    log "[dry-run] Pre-flight checks passed. Would download, verify, and install $DEST_PATH."
    exit 0
fi

# Create temporary directory
TMPDIR=$(mktemp -d -t lidb-install-XXXXXX)
cleanup() {
    rm -rf "$TMPDIR"
}
trap cleanup EXIT INT TERM

BINARY_URL="https://github.com/$REPO/releases/download/$INSTALL_TAG/$ASSET_NAME"
CHECKSUM_URL="https://github.com/$REPO/releases/download/$INSTALL_TAG/SHA256SUMS.txt"

log "Downloading binary from $BINARY_URL..."
if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$BINARY_URL" -o "$TMPDIR/$ASSET_NAME"
    curl -fsSL "$CHECKSUM_URL" -o "$TMPDIR/SHA256SUMS.txt"
else
    wget -q "$BINARY_URL" -O "$TMPDIR/$ASSET_NAME"
    wget -q "$CHECKSUM_URL" -O "$TMPDIR/SHA256SUMS.txt"
fi

log "Verifying SHA256 checksum..."
cd "$TMPDIR"
EXPECTED_SHA=$(grep "$ASSET_NAME" SHA256SUMS.txt | awk '{print $1}')
if [ -z "$EXPECTED_SHA" ]; then
    error "Checksum for $ASSET_NAME not found in release SHA256SUMS.txt."
fi

ACTUAL_SHA=$($SHA_CMD "$ASSET_NAME" | awk '{print $1}')
if [ "$EXPECTED_SHA" != "$ACTUAL_SHA" ]; then
    error "Checksum verification failed! Expected: $EXPECTED_SHA, Got: $ACTUAL_SHA"
fi
log "Checksum verified: $ACTUAL_SHA"

# Ensure destination directory exists
mkdir -p "$TARGET_DIR"

log "Installing binary to $DEST_PATH..."
cp "$TMPDIR/$ASSET_NAME" "$DEST_PATH"
chmod 0755 "$DEST_PATH"

log "Successfully installed $BINARY_NAME $INSTALL_TAG to $DEST_PATH!"
if ! printf "%s" ":$PATH:" | grep -q ":$TARGET_DIR:"; then
    log "Note: $TARGET_DIR is not currently in your PATH."
    log "Add it to your PATH by adding this line to your shell profile (~/.bashrc or ~/.zshrc):"
    log "  export PATH=\"\$PATH:$TARGET_DIR\""
fi
EOF