# Getting Started with Himalayas Browser

Welcome to Himalayas Browser - the world's first truly agent-native browser platform.

**Table of Contents**
- [What is Himalayas?](#what-is-himalayas)
- [Installation](#installation)
- [First Launch](#first-launch)
- [Basic Usage](#basic-usage)
- [Key Features](#key-features)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)

---

## What is Himalayas?

Himalayas Browser is **not a mainstream-browser competitor**. It's a fundamentally different architecture:

```
Traditional Browser (mainstream browsers like Firefox, Safari)
└─ GUI-first design
   └─ APIs and automation as afterthought
   └─ Agents are visitors, not citizens

Himalayas Browser (Agent-Native OS)
└─ Runtime-first design
   └─ Agents are native, GUI is optional
   └─ Autonomous execution with human oversight
   └─ Privacy-first, security-first architecture
```

### Core Differences

| Feature | Mainstream Browser | Himalayas |
|---------|--------|-----------|
| **Primary Interface** | GUI (visual browser) | Runtime (headless-first) |
| **Agents** | Bolt-on feature | Native citizens |
| **Automation** | Puppeteer/Selenium | Native APIs |
| **Privacy** | Settings → privacy | Architecture → privacy |
| **Cookies** | All 3rd-party allowed | Blocked by default |
| **Tracking** | Allow, then block | Block by default |
| **Permissions** | Persistent | Auto-expiring |
| **Memory** | Persistent cache | Ephemeral by default |

### Perfect For

✅ **Government workflows** - DigiLocker, eSign, Aadhaar automation  
✅ **Enterprise RPA** - Workflow automation, process efficiency  
✅ **Privacy-conscious users** - No tracking, no fingerprinting  
✅ **Developers** - Headless API-first design  
✅ **Research** - Transparent, auditable operations

---

## Installation

No packaged installer (.msi/.dmg/.deb/.rpm) or package-manager release
(Chocolatey, apt) is published yet — this initial release is
build-from-source/cargo-only. The `packaging/*-install.sh`/`.ps1` scripts in
this repo are scaffolding for that future release flow (they download a
release artifact that doesn't exist yet); don't run them until a tagged
release actually exists. Two ways that work today:

### Homebrew (macOS/Linux)
```bash
brew tap mullassery/himalayas-browser https://github.com/Mullassery/Himalayas-Browser
brew install --HEAD himalayas
```

This builds from source via `cargo` (see `Formula/himalayas.rb`) — there's
no hosted binary yet, so `--HEAD` (build from the repo's default branch) is
required until a tagged release exists.

### Install from Source

```bash
# Clone repository
git clone https://github.com/Mullassery/Himalayas-Browser.git
cd Himalayas-Browser

# Build
cargo build --release

# Run
./target/release/himalayas

# Or install system-wide
cargo install --path . --locked
```

**Requirements**: Rust 1.75+, Cargo

---

## First Launch

### Launch Himalayas

```bash
himalayas
```

Or:
- **Windows**: Start Menu → Himalayas Browser
- **macOS**: Spotlight → Himalayas
- **Linux**: Application Menu → Himalayas Browser

### Initial Setup

On first launch, Himalayas will:

1. **Detect your device** - RAM, CPU, GPU, storage
2. **Select optimal profile** - Auto-selects best performance profile:
   - UltraCapability (24+ GB RAM, GPU)
   - HighCapability (8-16 GB RAM)
   - Standard (4-8 GB RAM)
   - LowMemory (2-4 GB RAM)
   - PowerSaver (<2 GB RAM)

3. **Create config directory**
   - Linux: `~/.config/himalayas/`
   - macOS: `~/Library/Application Support/Himalayas/`
   - Windows: `%APPDATA%\Himalayas\`

4. **Open welcome page** with quick-start guides

### Configuration

Edit `config.toml` in your config directory:

```toml
[browser]
# Start with specific profile (auto-detected if not set)
startup_profile = "Standard"

# Start in private mode by default
private_by_default = true

# Enable agent runtime
enable_agents = true

[security]
# Block tracking by default
block_tracking = true

# Disable third-party cookies
disable_third_party_cookies = true

# Enable sandbox for all processes
enable_sandbox = true

[privacy]
# Minimal forensic traces (ephemeral cache)
minimal_forensics = true

# Automatic cleanup (delete data on exit)
auto_cleanup = true

# Ephemeral data (nothing persisted by default)
ephemeral_data = true

[ai]
# Enable local AI models (if device capable)
enable_local_models = true

# Fallback to cloud API if local models unavailable
enable_cloud_fallback = true
```

---

## Basic Usage

### Opening a Website

```bash
# Open homepage
himalayas

# Open specific URL
himalayas https://example.com

# Multiple URLs in new tabs
himalayas https://site1.com https://site2.com
```

### Keyboard Shortcuts

#### Navigation
| Shortcut | Action |
|----------|--------|
| `Ctrl+L` (Cmd+L on macOS) | Focus address bar |
| `Ctrl+T` | New tab |
| `Ctrl+W` | Close tab |
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |
| `Alt+←` (Cmd+[) | Back |
| `Alt+→` (Cmd+]) | Forward |
| `Ctrl+R` (Cmd+R) | Reload |
| `Ctrl+Shift+R` | Hard reload |

#### Page Functions
| Shortcut | Action |
|----------|--------|
| `Ctrl+F` (Cmd+F) | Find in page |
| `Ctrl+P` (Cmd+P) | Print |
| `Ctrl+S` (Cmd+S) | Save page |
| `Ctrl+U` (Cmd+U) | View source |
| `F12` | Developer tools |
| `Ctrl+Shift+C` (Cmd+Shift+C) | Inspect element |

#### AI Features
| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+S` (Cmd+Shift+S) | Summarize page |
| `Ctrl+Space` (Cmd+Space) | AI assistant |
| `Ctrl+Shift+P` (Cmd+Shift+P) | Command palette |

### Command Palette

Open with `Ctrl+Shift+P` (or `Cmd+Shift+P` on macOS).

Type to search for commands:

```
# Navigation
"open tab"              → Open new tab
"go home"              → Go to homepage
"show history"         → Show browsing history
"show bookmarks"       → Show bookmarks

# AI Actions
"summarize"            → Summarize current page
"translate"            → Translate page
"fact check"           → Verify facts on page
"extract entities"     → Extract key information

# Document
"save as PDF"          → Export page to PDF
"print"                → Print page
"screenshot"           → Take screenshot

# Workspace
"create workspace"     → Create new workspace
"switch workspace"     → Switch workspace

# Settings
"clear cache"          → Clear browsing data
"settings"             → Open preferences
"keyboard shortcuts"   → Show all shortcuts
```

---

## Key Features

### 1. Privacy by Default

**No tracking, no cookies, no fingerprinting**

```
✅ Third-party cookies: Blocked
✅ Tracking pixels: Blocked
✅ Local storage: Not persisted
✅ Service workers: Sandboxed
✅ Fingerprinting: Prevented
✅ WebRTC: Proxied
```

**Every session is private** unless explicitly enabled.

### 2. Document Intelligence

**View and interact with documents**

Supported formats:
- PDF (render, annotate, search, extract)
- Word (DOCX)
- Excel (XLSX)
- PowerPoint (PPTX)
- Text (TXT, RTF)
- ODF (ODT, ODS)

Features:
```bash
# Annotate documents
Right-click → Highlight, Note, Circle, Underline

# Extract information
Right-click → Extract entities, OCR text, Extract tables

# AI operations
Right-click → Summarize document, Generate Q&A
```

### 3. Adaptive Intelligence

**Browser adapts to your device**

Himalayas auto-detects:
- Available RAM (GB)
- CPU cores
- GPU presence
- Storage space
- Network speed

Then automatically:
- Loads/unloads features
- Adjusts quality settings
- Manages memory
- Optimizes performance

No restart needed - happens at runtime.

### 4. Spatial Intelligence

**Location-aware features** (India Stack integration optional)

Features:
- Multi-GNSS support (GPS, NavIC, BeiDou, Galileo, GLONASS, QZSS)
- Location memory (places you've visited)
- Spoofing detection
- Trajectory analysis

### 5. AI Assistant

**Keyboard accessible** (`Ctrl+Space` or `Cmd+Space`)

Commands:
```
Summarize      → Create brief summary of page
Explain        → Explain in simple terms
Translate      → Translate to another language
Fact Check     → Verify claims
Extract        → Pull key information
Define         → Look up definitions
Generate Code  → Write code snippets
Compare        → Compare two documents
```

### 6. Workspaces

**Organize browsing by context**

```bash
# Create workspace
Ctrl+Shift+P → "create workspace"
Name: "Research" / "Work" / "Shopping"

# Switch workspace
Ctrl+Shift+P → "switch workspace"

# Move tab to workspace
Right-click tab → "Move to workspace"
```

Each workspace maintains:
- Separate tabs
- Independent history
- Isolated permissions
- Private data storage

### 7. Headless Mode

**No GUI - perfect for automation**

```bash
# Run headless (no window)
himalayas --headless https://example.com

# With API server
himalayas --api-server 8000
curl http://localhost:8000/tabs
curl http://localhost:8000/navigate -X POST -d '{"url":"https://example.com"}'
```

---

## Advanced Features

### Creating Macros

Record and replay workflows:

```bash
Ctrl+Alt+J    → Start recording
[perform actions]
Ctrl+Alt+J    → Stop recording
Ctrl+Shift+R  → Replay macro
```

### Device Profiles

Manually select profile (or auto-detect):

**Edit config.toml**:
```toml
[browser]
startup_profile = "Standard"  # or "UltraCapability", "HighCapability", "LowMemory", "PowerSaver"
```

**Profile Capabilities**:

| Profile | RAM | Features | AI Models | GPU |
|---------|-----|----------|-----------|-----|
| **UltraCapability** | 24+ GB | All | Local | Yes |
| **HighCapability** | 8-16 GB | Most | Local | Yes |
| **Standard** | 4-8 GB | Core | Cloud | Optional |
| **LowMemory** | 2-4 GB | Essential | Cloud | No |
| **PowerSaver** | <2 GB | Minimal | None | No |

### Developer Mode

Enable for DevTools and advanced features:

```toml
[developer]
enabled = true
show_dev_menu = true
enable_extensions = true
```

---

## Customization

### Custom Shortcuts

Edit `config.toml`:

```toml
[shortcuts]
# Override default shortcuts
"Ctrl+H" = "show_history"
"Ctrl+B" = "show_bookmarks"
"Alt+D" = "show_downloads"
```

### Custom Search Engines

```toml
[search_engines]
# Set default
default = "google"

# Add custom
[search_engines.github]
url = "https://github.com/search?q={query}"
shortcut = "gh"

# Usage:
# Type: gh my_repo
# Searches: https://github.com/search?q=my_repo
```

### Custom Themes

```toml
[ui]
theme = "dark"  # or "light", "auto"
accent_color = "#0066cc"
font_size = 14
```

---

## Troubleshooting

### Browser Won't Start

```bash
# Check logs
himalayas --verbose

# Reset to defaults
himalayas --reset-defaults

# Check version
himalayas --version
```

### Memory Usage Too High

1. Lower device profile: `startup_profile = "LowMemory"`
2. Disable local AI: `enable_local_models = false`
3. Enable auto-cleanup: `auto_cleanup = true`
4. Clear cache: `Ctrl+Shift+P` → "clear cache"

### Slow Performance

1. Close unused tabs (Ctrl+W)
2. Disable extensions
3. Check CPU/GPU with DevTools (F12)
4. Lower render quality: `render_quality = "low"`

### Crashed on Startup

```bash
# Remove config file (resets to defaults)
rm -rf ~/.config/himalayas/config.toml  # Linux
rm -rf ~/Library/Application\ Support/Himalayas/config.toml  # macOS
rmdir %APPDATA%\Himalayas\config.toml  # Windows (PowerShell)

# Try again
himalayas
```

### Keyboard Shortcuts Not Working

Ensure your shell isn't intercepting shortcuts:

**Bash**: Edit `.bashrc`
```bash
# Disable conflicting bindings
bind -r '\C-s'  # frees Ctrl+S
bind -r '\C-q'  # frees Ctrl+Q
```

**Fish**: Edit `~/.config/fish/config.fish`
```fish
bind --erase \cs  # frees Ctrl+S
```

---

## Command Reference

### CLI Options

```bash
himalayas [OPTIONS] [URL]

OPTIONS
  --help                Show help
  --version             Show version
  --new-window          Open new window
  --new-private-window  Open private window
  --headless            Run without GUI
  --api-server PORT     Start API server on port
  --profile PROFILE     Use specific profile
  --config-dir          Show config directory
  --data-dir            Show data directory
  --verbose             Show debug output
  --reset-defaults      Reset to factory settings

EXAMPLES
  himalayas                              # Start normally
  himalayas https://example.com          # Open URL
  himalayas --new-private-window         # Private mode
  himalayas --headless --api-server 8000 # Headless API
  himalayas --profile LowMemory           # Low-memory device
```

---

## Next Steps

### Learn More

- [Usage Guide](./USAGE.md) - Detailed features & workflows
- [Configuration](./INSTALLATION.md#configuration) - All config options
- [Keyboard & Trackpad](./KEYBOARD_TRACKPAD_SPEC.md) - Advanced input (Phase 6)
- [Architecture](./PLATFORM_SPECIFIC.md) - Technical details

### Try Demos

```bash
# Open demo page
himalayas https://github.com/Mullassery/Himalayas-Browser

# Try command palette
Ctrl+Shift+P

# Try AI assistant
Ctrl+Space

# Try document viewer
himalayas --headless https://example.com/document.pdf
```

### Get Help

- **Issues**: [GitHub Issues](https://github.com/Mullassery/Himalayas-Browser/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Mullassery/Himalayas-Browser/discussions)
- **Email**: mullassery@gmail.com

---

## Tips & Tricks

### Productivity

```
💡 Use workspaces to organize by context (Work/Personal/Research)
💡 Keyboard shortcuts for everything - avoid mouse
💡 Command palette for quick access to any feature
💡 Create macros for repetitive workflows
💡 Use AI assistant for quick answers
```

### Performance

```
💡 Enable auto-cleanup to reclaim memory
💡 Use LowMemory profile on constrained devices
💡 Close unused tabs to free resources
💡 Disable local models if cloud is available
```

### Security

```
💡 Always use private window mode (default)
💡 Check permission settings (Ctrl+Shift+P → "permissions")
💡 Review active agents (Ctrl+Shift+P → "agents")
💡 Export and verify browsing history regularly
```

---

**Happy browsing! 🏔️**

For more information, see the [README](./README.md) or [Documentation](./INSTALLATION.md).
