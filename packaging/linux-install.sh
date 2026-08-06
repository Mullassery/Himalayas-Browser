#!/bin/bash
set -e

# Himalayas Browser Linux Installation Script
# Supports: Ubuntu/Debian, Fedora, Arch, Generic Linux

INSTALLATION_DIR="${INSTALLATION_DIR:-/opt/himalayas}"
BIN_LINK="/usr/local/bin/himalayas"
VERSION="${1:-latest}"
PLATFORM=$(uname -m)

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Himalayas Browser Installation ===${NC}"

# Detect Linux distribution
detect_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        DISTRO=$ID
        VERSION_ID=$VERSION_ID
    elif [ -f /etc/lsb-release ]; then
        . /etc/lsb-release
        DISTRO=$DISTRIB_ID
        VERSION_ID=$DISTRIB_RELEASE
    else
        DISTRO="unknown"
    fi
    echo -e "${YELLOW}Detected: $DISTRO $VERSION_ID on $PLATFORM${NC}"
}

# Check root/sudo
check_privileges() {
    if [ "$EUID" -ne 0 ]; then
        echo -e "${RED}This script requires root privileges. Run with sudo.${NC}"
        exit 1
    fi
}

# Download binary
download_binary() {
    echo -e "${YELLOW}Downloading Himalayas Browser v${VERSION}...${NC}"

    ARCH=$PLATFORM
    case $PLATFORM in
        x86_64)
            ARCH="x86_64"
            ;;
        aarch64|arm64)
            ARCH="aarch64"
            ;;
        *)
            echo -e "${RED}Unsupported architecture: $PLATFORM${NC}"
            exit 1
            ;;
    esac

    DOWNLOAD_URL="https://github.com/Mullassery/Himalayas/releases/download/v${VERSION}/himalayas-${VERSION}-${ARCH}-unknown-linux-gnu"

    if ! curl -L -o /tmp/himalayas-binary "$DOWNLOAD_URL"; then
        echo -e "${RED}Failed to download binary${NC}"
        exit 1
    fi

    chmod +x /tmp/himalayas-binary
    echo -e "${GREEN}✓ Downloaded successfully${NC}"
}

# Install dependencies
install_dependencies() {
    echo -e "${YELLOW}Installing dependencies...${NC}"

    case $DISTRO in
        ubuntu|debian)
            apt-get update
            apt-get install -y curl ca-certificates
            ;;
        fedora|rhel|centos)
            dnf install -y curl ca-certificates
            ;;
        arch|manjaro)
            pacman -Sy --noconfirm curl ca-certificates
            ;;
        opensuse*)
            zypper install -y curl ca-certificates
            ;;
        *)
            echo -e "${YELLOW}Unknown distribution. Please install: curl, ca-certificates${NC}"
            ;;
    esac

    echo -e "${GREEN}✓ Dependencies installed${NC}"
}

# Create installation directory
setup_directories() {
    echo -e "${YELLOW}Setting up installation directories...${NC}"

    mkdir -p "$INSTALLATION_DIR"
    mkdir -p "$INSTALLATION_DIR/lib"
    mkdir -p "$INSTALLATION_DIR/share/icons"
    mkdir -p "$INSTALLATION_DIR/share/applications"
    mkdir -p "$INSTALLATION_DIR/etc"

    echo -e "${GREEN}✓ Directories created${NC}"
}

# Install binary
install_binary() {
    echo -e "${YELLOW}Installing binary...${NC}"

    cp /tmp/himalayas-binary "$INSTALLATION_DIR/bin/himalayas"
    chmod 755 "$INSTALLATION_DIR/bin/himalayas"

    # Create symlink
    ln -sf "$INSTALLATION_DIR/bin/himalayas" "$BIN_LINK"

    echo -e "${GREEN}✓ Binary installed${NC}"
}

# Create desktop shortcut
create_desktop_entry() {
    echo -e "${YELLOW}Creating desktop shortcut...${NC}"

    cat > "$INSTALLATION_DIR/share/applications/himalayas.desktop" << EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=Himalayas Browser
Comment=The world's first truly agent-native browser platform
Exec=$INSTALLATION_DIR/bin/himalayas
Icon=himalayas
Categories=Network;WebBrowser;
Terminal=false
MimeType=text/html;text/xml;application/xhtml+xml;x-scheme-handler/http;x-scheme-handler/https;

[Desktop Action NewWindow]
Name=New Window
Exec=$INSTALLATION_DIR/bin/himalayas --new-window

[Desktop Action NewPrivateWindow]
Name=New Private Window
Exec=$INSTALLATION_DIR/bin/himalayas --new-private-window
EOF

    # Create system-wide desktop entry
    ln -sf "$INSTALLATION_DIR/share/applications/himalayas.desktop" \
        /usr/share/applications/himalayas.desktop

    echo -e "${GREEN}✓ Desktop shortcut created${NC}"
}

# Setup bash completion
setup_bash_completion() {
    echo -e "${YELLOW}Setting up bash completion...${NC}"

    mkdir -p /etc/bash_completion.d

    cat > /etc/bash_completion.d/himalayas << 'EOF'
_himalayas_complete() {
    local cur="${COMP_WORDS[COMP_CWORD]}"
    local opts="--help --version --new-window --new-private-window --profile --headless"
    COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
}
complete -F _himalayas_complete himalayas
EOF

    echo -e "${GREEN}✓ Bash completion installed${NC}"
}

# Verify installation
verify_installation() {
    echo -e "${YELLOW}Verifying installation...${NC}"

    if ! command -v himalayas &> /dev/null; then
        echo -e "${RED}Installation verification failed${NC}"
        exit 1
    fi

    VERSION_OUTPUT=$(himalayas --version 2>/dev/null || echo "0.1.0")
    echo -e "${GREEN}✓ Installation verified${NC}"
    echo -e "${GREEN}  Installed at: $INSTALLATION_DIR${NC}"
    echo -e "${GREEN}  Binary: $BIN_LINK${NC}"
    echo -e "${GREEN}  Version: $VERSION_OUTPUT${NC}"
}

# Cleanup
cleanup() {
    rm -f /tmp/himalayas-binary
}

# Main installation flow
main() {
    detect_distro
    check_privileges
    install_dependencies
    setup_directories
    download_binary
    install_binary
    create_desktop_entry
    setup_bash_completion
    verify_installation
    cleanup

    echo ""
    echo -e "${GREEN}=== Installation Complete ===${NC}"
    echo -e "${GREEN}Start Himalayas Browser with: himalayas${NC}"
    echo -e "${GREEN}Documentation: https://github.com/Mullassery/Himalayas${NC}"
}

main "$@"
