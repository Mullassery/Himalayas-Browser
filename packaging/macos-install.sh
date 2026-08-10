#!/bin/bash
set -e

# Himalayas Browser macOS Installation Script
# Supports: macOS 11.0+ (Intel & Apple Silicon)
#
# NOT YET FUNCTIONAL: downloads a DMG from a GitHub Releases tag that does
# not exist yet (no tagged release has been published). This is scaffolding
# for that future release flow, not something to run today. Until then, use
# `brew install --HEAD himalayas` (see Formula/himalayas.rb) or build from
# source directly — see docs/GETTING_STARTED.md.

INSTALLATION_DIR="/Applications/Himalayas.app"
BIN_LINK="/usr/local/bin/himalayas"
VERSION="${1:-latest}"
ARCH=$(uname -m)

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}=== Himalayas Browser Installation (macOS) ===${NC}"

# Check macOS version
check_macos_version() {
    MACOS_VERSION=$(sw_vers -productVersion | cut -d'.' -f1)
    if [ "$MACOS_VERSION" -lt 11 ]; then
        echo -e "${RED}macOS 11.0 or later is required${NC}"
        exit 1
    fi
    echo -e "${YELLOW}macOS $MACOS_VERSION detected${NC}"
}

# Check architecture
check_architecture() {
    if [ "$ARCH" != "arm64" ] && [ "$ARCH" != "x86_64" ]; then
        echo -e "${RED}Unsupported architecture: $ARCH${NC}"
        exit 1
    fi
    echo -e "${YELLOW}Architecture: $ARCH${NC}"
}

# Check if running on Apple Silicon
is_apple_silicon() {
    [ "$ARCH" = "arm64" ]
}

# Download DMG
download_dmg() {
    echo -e "${YELLOW}Downloading Himalayas Browser v${VERSION}...${NC}"

    DOWNLOAD_URL="https://github.com/Mullassery/Himalayas/releases/download/v${VERSION}/himalayas-${VERSION}-universal.dmg"

    if ! curl -L -o /tmp/himalayas.dmg "$DOWNLOAD_URL"; then
        echo -e "${RED}Failed to download DMG${NC}"
        exit 1
    fi

    echo -e "${GREEN}✓ Downloaded successfully${NC}"
}

# Mount and install from DMG
install_from_dmg() {
    echo -e "${YELLOW}Mounting DMG...${NC}"

    # Mount the DMG
    MOUNT_POINT=$(mktemp -d)
    hdiutil attach /tmp/himalayas.dmg -mountpoint "$MOUNT_POINT" -nobrowse

    # Copy app to Applications
    echo -e "${YELLOW}Installing application...${NC}"
    cp -r "$MOUNT_POINT/Himalayas/Himalayas.app" "$INSTALLATION_DIR"

    # Unmount
    hdiutil detach "$MOUNT_POINT"
    rm -rf "$MOUNT_POINT"
    rm -f /tmp/himalayas.dmg

    echo -e "${GREEN}✓ Application installed${NC}"
}

# Create binary symlink
create_symlink() {
    echo -e "${YELLOW}Creating command-line symlink...${NC}"

    # Get the actual binary location within the app bundle
    BINARY_PATH="$INSTALLATION_DIR/Contents/MacOS/himalayas"

    # Create symlink in /usr/local/bin
    mkdir -p /usr/local/bin
    ln -sf "$BINARY_PATH" "$BIN_LINK"

    echo -e "${GREEN}✓ Symlink created at $BIN_LINK${NC}"
}

# Trust the app (handle quarantine attributes)
trust_app() {
    echo -e "${YELLOW}Trusting application...${NC}"

    xattr -d com.apple.quarantine "$INSTALLATION_DIR" 2>/dev/null || true
    chmod -R 755 "$INSTALLATION_DIR"

    echo -e "${GREEN}✓ Application trusted${NC}"
}

# Setup Homebrew cask (optional)
setup_homebrew() {
    if command -v brew &> /dev/null; then
        echo -e "${YELLOW}Homebrew detected. Creating tap...${NC}"

        mkdir -p "$(brew --repo)/Library/Taps/mullassery"
        # User can add to Homebrew later
    fi
}

# Verify installation
verify_installation() {
    echo -e "${YELLOW}Verifying installation...${NC}"

    if [ ! -d "$INSTALLATION_DIR" ]; then
        echo -e "${RED}Installation verification failed${NC}"
        exit 1
    fi

    if [ ! -L "$BIN_LINK" ]; then
        echo -e "${RED}Command-line symlink verification failed${NC}"
        exit 1
    fi

    echo -e "${GREEN}✓ Installation verified${NC}"
}

# Create desktop/Launchpad entry
create_launchpad_entry() {
    echo -e "${YELLOW}Updating Launchpad...${NC}"

    # Force Spotlight to index the application
    mdimport "$INSTALLATION_DIR" 2>/dev/null || true

    echo -e "${GREEN}✓ Launchpad updated${NC}"
}

# Main installation flow
main() {
    check_macos_version
    check_architecture

    if is_apple_silicon; then
        echo -e "${YELLOW}Apple Silicon detected - using native ARM64 binary${NC}"
    fi

    download_dmg
    install_from_dmg
    trust_app
    create_symlink
    create_launchpad_entry
    setup_homebrew
    verify_installation

    echo ""
    echo -e "${GREEN}=== Installation Complete ===${NC}"
    echo -e "${GREEN}Start Himalayas Browser with:${NC}"
    echo -e "${GREEN}  • himalayas (command line)${NC}"
    echo -e "${GREEN}  • open -a Himalayas (native app)${NC}"
    echo -e "${GREEN}  • Launchpad → Himalayas${NC}"
    echo -e "${GREEN}Documentation: https://github.com/Mullassery/Himalayas${NC}"
}

main "$@"
