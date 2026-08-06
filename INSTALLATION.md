# Himalayas Browser Installation Guide

The world's first truly agent-native browser platform. Available for Windows, macOS, and Linux.

## Quick Start

### Windows (x86_64 & ARM64)

#### Option 1: MSI Installer (Recommended)
1. Download the MSI installer from [GitHub Releases](https://github.com/Mullassery/Himalayas/releases)
2. Double-click `himalayas-x.y.z-x86_64.msi`
3. Follow the installation wizard
4. Launch from Start Menu or desktop shortcut

#### Option 2: PowerShell Script
```powershell
# Run as Administrator
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
iex (New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/Mullassery/Himalayas/main/packaging/windows-install.ps1')
```

#### Option 3: Portable ZIP
1. Download `himalayas-x.y.z-x86_64.zip`
2. Extract to any location
3. Run `himalayas.exe`

#### Uninstall
- Control Panel → Programs → Programs and Features → Himalayas Browser
- Or run: `C:\Program Files\Himalayas\Uninstall.ps1`

---

### macOS (Intel & Apple Silicon)

#### Option 1: DMG Installer (Recommended)
1. Download `himalayas-x.y.z-universal.dmg` from [GitHub Releases](https://github.com/Mullassery/Himalayas/releases)
2. Double-click the DMG file
3. Drag Himalayas to Applications folder
4. Launch from Applications or Spotlight

#### Option 2: Shell Script
```bash
bash <(curl -fsSL https://raw.githubusercontent.com/Mullassery/Himalayas/main/packaging/macos-install.sh)
```

#### Option 3: Homebrew (when available)
```bash
brew install mullassery/himalayas/himalayas
```

#### Command-Line Usage
```bash
# Start Himalayas
himalayas

# Open specific URL
himalayas https://example.com

# New private window
himalayas --new-private-window

# Specific profile
himalayas --profile work
```

#### Uninstall
```bash
rm -rf /Applications/Himalayas.app
rm /usr/local/bin/himalayas
```

---

### Linux (x86_64 & ARM64)

#### Option 1: Automated Install Script (Recommended)
```bash
sudo bash <(curl -fsSL https://raw.githubusercontent.com/Mullassery/Himalayas/main/packaging/linux-install.sh)
```

This script automatically:
- Detects your Linux distribution
- Installs required dependencies
- Downloads and installs the binary
- Creates desktop shortcuts
- Sets up bash completion

#### Option 2: Manual Installation

##### Ubuntu/Debian
```bash
# Download
wget https://github.com/Mullassery/Himalayas/releases/download/v0.1.0/himalayas_0.1.0_amd64.deb

# Install
sudo dpkg -i himalayas_0.1.0_amd64.deb
sudo apt-get install -f  # Install missing dependencies if any

# Run
himalayas
```

##### Fedora/RHEL
```bash
# Download and install
sudo dnf install https://github.com/Mullassery/Himalayas/releases/download/v0.1.0/himalayas_0.1.0_x86_64.rpm

# Run
himalayas
```

##### Arch Linux
```bash
# Download and install
yay -S himalayas  # or manual installation from releases

# Run
himalayas
```

##### Generic Linux
```bash
# Download binary
wget https://github.com/Mullassery/Himalayas/releases/download/v0.1.0/himalayas-0.1.0-x86_64-unknown-linux-gnu

# Make executable and install
chmod +x himalayas-0.1.0-x86_64-unknown-linux-gnu
sudo mv himalayas-0.1.0-x86_64-unknown-linux-gnu /usr/local/bin/himalayas

# Verify
himalayas --version
```

#### Command-Line Usage
```bash
# Start Himalayas
himalayas

# Open specific URL
himalayas https://example.com

# New private window
himalayas --new-private-window

# Headless mode (for automation)
himalayas --headless https://example.com

# Bash completion
himalayas [TAB]
```

#### Uninstall
```bash
# Ubuntu/Debian
sudo apt-get remove himalayas

# Fedora
sudo dnf remove himalayas

# Arch
yay -R himalayas

# Generic Linux
sudo rm /usr/local/bin/himalayas
sudo rm -rf /opt/himalayas
```

---

## System Requirements

### Windows
- **OS**: Windows 10 or later
- **Processor**: x86_64 or ARM64
- **RAM**: 2 GB minimum, 8 GB recommended
- **Storage**: 500 MB available space
- **Administrator privileges**: Required for installation

### macOS
- **OS**: macOS 11.0 or later
- **Processor**: Intel (x86_64) or Apple Silicon (ARM64)
- **RAM**: 2 GB minimum, 8 GB recommended
- **Storage**: 500 MB available space
- **Xcode Command Line Tools**: Optional (for development)

### Linux
- **OS**: Ubuntu 20.04+, Debian 11+, Fedora 35+, Arch, or other modern distributions
- **Processor**: x86_64 or ARM64
- **RAM**: 2 GB minimum, 8 GB recommended
- **Storage**: 500 MB available space
- **Dependencies**: curl, ca-certificates (automatically installed)

---

## Configuration

### Config Files Location

**Windows**
```
%APPDATA%\Himalayas\config.toml
%APPDATA%\Himalayas\profiles\
```

**macOS**
```
~/Library/Application Support/Himalayas/config.toml
~/Library/Application Support/Himalayas/profiles/
```

**Linux**
```
~/.config/himalayas/config.toml
~/.local/share/himalayas/profiles/
```

### Default Configuration
```toml
[browser]
startup_profile = "Standard"
private_by_default = true
enable_agents = true

[security]
block_tracking = true
disable_third_party_cookies = true
enable_sandbox = true

[privacy]
minimal_forensics = true
auto_cleanup = true
ephemeral_data = true

[ai]
enable_local_models = true
enable_cloud_fallback = true
```

---

## Troubleshooting

### Windows

**MSI Installation Fails**
```powershell
# Try with detailed logging
msiexec /i himalayas.msi /l*v install.log
```

**Command Not Found**
- Ensure installation directory is in PATH
- Restart terminal after installation
- Check: `echo $env:Path`

### macOS

**"Himalayas cannot be opened" Error**
```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine /Applications/Himalayas.app
```

**Permission Denied**
```bash
# Fix permissions
chmod -R 755 /Applications/Himalayas.app
```

**Apple Silicon Issues**
- Ensure you're using the universal or arm64 binary
- Check: `himalayas --version`

### Linux

**Permission Denied**
```bash
# Ensure binary is executable
chmod +x ~/.local/bin/himalayas
```

**Library Not Found**
```bash
# Install development libraries
sudo apt-get install libssl-dev libfontconfig1-dev  # Ubuntu/Debian
sudo dnf install openssl-devel fontconfig-devel     # Fedora
```

**Desktop Icon Not Appearing**
```bash
# Update Freedesktop database
update-desktop-database ~/.local/share/applications
```

---

## Verification

### Verify Installation
```bash
# Check version
himalayas --version

# Check binary location
which himalayas

# Verify permissions
ls -la $(which himalayas)

# Check configuration
himalayas --config-dir
```

### Test Connectivity
```bash
# Launch with verbose logging
RUST_LOG=debug himalayas
```

---

## Advanced Installation

### Building from Source
```bash
# Clone repository
git clone https://github.com/Mullassery/Himalayas.git
cd Himalayas

# Build release binary
cargo build --release

# Binary location
./target/release/himalayas
```

### Cross-Compilation
```bash
# Install cross
cargo install cross

# Build for different target
cross build --release --target aarch64-unknown-linux-gnu
cross build --release --target x86_64-pc-windows-gnu
```

### Docker Installation
```dockerfile
FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/himalayas /usr/local/bin/
ENTRYPOINT ["himalayas"]
```

---

## Platform-Specific Notes

### Windows
- Native Windows Sandbox support for enhanced security
- DirectML GPU acceleration
- Windows Update integration for automatic updates
- Group Policy support for enterprise deployment

### macOS
- Metal GPU acceleration
- macOS Keychain integration
- Gatekeeper and code signing support
- Universal binary for both Intel and Apple Silicon

### Linux
- Wayland and X11 support
- Systemd integration
- XDG desktop portal integration
- Distro-specific package managers

---

## Getting Started

After installation, run Himalayas:
```bash
himalayas
```

### First Launch
1. Review privacy settings
2. Configure device profile (Standard/LowMemory/PowerSaver)
3. Set up agent permissions if desired
4. Enable/disable tracking protection
5. Configure workspaces (Research/Coding/Writing)

### Command-Line Options
```bash
himalayas [OPTIONS] [URL]

OPTIONS:
  --help                    Show help message
  --version                 Show version
  --new-window              Open new window
  --new-private-window      Open private window
  --profile PROFILE         Load specific profile
  --headless                Run without UI
  --config-dir              Show config directory
  --data-dir                Show data directory
  --reset-defaults          Reset to defaults
```

---

## Uninstall

### Completely Remove Himalayas

**Windows**
```powershell
# Remove application
Remove-Item -Recurse -Force "C:\Program Files\Himalayas"

# Remove user data
Remove-Item -Recurse -Force "$env:APPDATA\Himalayas"
```

**macOS**
```bash
# Remove application
rm -rf /Applications/Himalayas.app

# Remove user data
rm -rf ~/Library/Application\ Support/Himalayas
rm ~/.zprofile  # if modified for PATH
```

**Linux**
```bash
# Remove binary
sudo rm /usr/local/bin/himalayas

# Remove application data
rm -rf ~/.config/himalayas
rm -rf ~/.local/share/himalayas

# Remove desktop entry
rm ~/.local/share/applications/himalayas.desktop
```

---

## Support

- **GitHub Issues**: https://github.com/Mullassery/Himalayas/issues
- **Documentation**: https://github.com/Mullassery/Himalayas/wiki
- **Email**: mullassery@gmail.com

---

## License

Proprietary License - Free to use with explicit attribution. See LICENSE file for details.

---

## Version History

### v0.1.0 (Current)
- Initial release with Phase 0-5 implementation
- 218 tests passing
- Multi-platform support (Windows, macOS, Linux)
- AI-native UI system
- Adaptive intelligence engine
- Document rendering platform
- Spatial intelligence with GNSS support
