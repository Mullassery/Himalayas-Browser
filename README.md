# 🏔️ Himalayas Browser

<div align="center">

**The world's first truly agent-native browser platform**

[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/Tests-218%2F218%20passing-brightgreen)](./src)
[![License](https://img.shields.io/badge/License-Proprietary%20(Free%20w%2F%20attribution)-blue)](#license)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](#platform-support)
[![Status](https://img.shields.io/badge/Status-Production%20Ready-success)]()

[Reaching the peak of autonomous computing.](#philosophy)

</div>

---

## 🎯 Vision

Himalayas is **not a Chrome competitor**. We're building the **Operating System for Agents**.

The browser as agents first-class citizens. AI infrastructure, not Chrome with AI bolted on.

```
Browser Evolution:
CLI → GUI Browser → Headless + API → Agent-Native OS ← You are here

Himalayas:
- Agents are native citizens, not guests
- Runtime-first design (GUI is optional)
- Autonomous execution with human approval gates
- Zero-trust security by architecture
- Privacy-first by design, not feature
```

---

## 🚀 What's Shipped

### Phase 0-5: Foundation Complete ✅

| Phase | Feature | Tests | Status |
|-------|---------|-------|--------|
| **0** | Health server, metrics, daemon | 20 | ✅ |
| **2.5** | Adaptive intelligence (5 profiles) | 74 | ✅ |
| **3** | Document platform (render, annotate, forms, AI) | 23 | ✅ |
| **4** | Spatial intelligence (GNSS, sensors, location memory) | 28 | ✅ |
| **5** | AI-native UI (context menus, adaptive) | 24 | ✅ |
| **6** | Keyboard & trackpad (planned Q4 2026) | — | 📋 |

**Total**: 218 tests, 100% passing | 1,500+ LOC UI | 7.8K LOC document platform

### Multi-Platform Distribution Ready ✅

| Platform | x86_64 | ARM64 | Installer | Status |
|----------|--------|-------|-----------|--------|
| **Windows** | ✅ | ✅ | MSI + ZIP | Ready |
| **macOS** | ✅ | ✅ | Universal DMG | Ready |
| **Linux** | ✅ | ✅ | DEB + RPM + Binary | Ready |

**Automated CI/CD**: GitHub Actions builds all platforms/architectures on tag push.

---

## 🏗️ Architecture Highlights

### Agent-Native Runtime

```rust
// Every agent is sandboxed, temporary, verified
agent {
    name: "license_renewal",
    scope: ["DigiLocker", "eSign", "RTO"],
    duration: Duration::minutes(15),
    capability: "government_workflow",
    approval: ApprovalRequired::Critical,
}
```

### 5 Adaptive Intelligence Profiles

Auto-detects device capabilities and adapts at runtime:

- **UltraCapability**: 24+ GB RAM, GPU, all features enabled
- **HighCapability**: 8-16 GB RAM, local AI models, most features
- **Standard**: 4-8 GB RAM, cloud AI, core features
- **LowMemory**: 2-4 GB RAM, minimal features, no background tasks
- **PowerSaver**: <2 GB RAM, essentials only, battery-optimized

Each profile loads/unloads features dynamically. Zero restart.

### Document Platform

- **Rendering**: PDF, DOCX, XLSX, PPTX, TXT, RTF, ODT, ODS
- **Annotations**: Highlight, underline, notes, circles, arrows
- **Forms**: Validation, type detection, auto-fill
- **AI**: Summarize, extract entities, Q&A, OCR, table detection

### Spatial Intelligence

- **Multi-GNSS**: GPS, NavIC (India), BeiDou, Galileo, GLONASS, QZSS
- **Sensor Fusion**: Weighted averaging (GNSS 50%, WiFi 25%, BLE 15%, IMU 10%)
- **Location Memory**: Persistent graph of visited places, trajectories
- **Spoofing Detection**: 5 anomaly types, automatic fallback

### AI-Native UI

- **8 Context Types**: Page, text, image, link, video, code, PDF, agent
- **20+ AI Actions**: Summarize, explain, translate, fact-check, debug code
- **9 Adaptive Menus**: Browser, AI, Navigate, Workspace, Security, Tools, Developer, Window, Help
- **Profile-based Adaptation**: Different menus for different device capabilities

---

## 💡 Key Philosophy

### Security is Architecture, Not Features

```
Traditional: Chrome + security features
Himalayas: Security is the foundation
```

**10 Security Layers** (not checkboxes):
1. Bot Sandboxing - Every agent isolated
2. Risk-Based Expiration - Auto-expiring permissions
3. Re-Auth Time-Bound - Sensitive actions require re-auth
4. Age-Based Safety - Child/Teen/Adult profiles with enforcement
5. Cybersecurity Policies - 17 core policies (origin isolation, prompt injection prevention)
6. Default-Deny Ads - Blocked by default, site opt-in, time-limited
7. Strict Cookie Isolation - First-party only, site isolation
8. Private-by-Default - Every session private unless explicitly enabled
9. Minimal Forensic Traces - Ephemeral by default, no permanent cache
10. Automatic Cleanup - Self-maintaining, permission expiration

### Privacy by Design

- **No persistent cookies** unless explicitly enabled
- **Ephemeral agents**: Spawned, used, destroyed
- **Memory-first architecture**: No unnecessary disk writes
- **Encrypted storage**: All persistent data encrypted
- **User control**: Every feature has off-switch, explicit opt-in

### India Stack Integration (Optional Feature)

Deep integration for government workflows:
- Aadhaar authentication
- DigiLocker document retrieval
- eSign digital signing
- Regional language support (Hindi, Tamil, Telugu, etc.)
- Government portal navigation

---

## 📦 Installation

### Quick Start

**Linux**
```bash
sudo bash <(curl -fsSL https://raw.githubusercontent.com/Mullassery/Himalayas/main/packaging/linux-install.sh)
```

**macOS**
```bash
bash <(curl -fsSL https://raw.githubusercontent.com/Mullassery/Himalayas/main/packaging/macos-install.sh)
```

**Windows (PowerShell as Admin)**
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
iex (New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/Mullassery/Himalayas/main/packaging/windows-install.ps1')
```

**Or download MSI/DMG/DEB** from [Releases](https://github.com/Mullassery/Himalayas/releases)

### From Source

```bash
git clone https://github.com/Mullassery/Himalayas.git
cd Himalayas
cargo build --release
./target/release/himalayas
```

---

## 🔧 Technology Stack

| Component | Technology | Why |
|-----------|-----------|-----|
| **Core** | Rust | Performance, memory safety, no GC |
| **Async** | Tokio | Non-blocking I/O, 1000+ concurrent tasks |
| **Parsing** | serde | Type-safe configuration, JSON/TOML/YAML |
| **GPU** | DirectX/Metal/Vulkan | Platform-native acceleration |
| **Crypto** | Standard libraries | No reinvention, security-first |
| **Testing** | Built-in | 218 comprehensive tests |

**Size**: ~50-100 MB per platform (compressed)  
**Memory**: 240-380 MB base (device-dependent)  
**Startup**: 700-950ms cold boot (device-dependent)  
**GPU**: Enabled by default (DirectX 12/Metal/Vulkan)

---

## 📊 Project Status

### ✅ Complete

- Architecture (22 documents, 80,000+ words)
- Phase 0-5 implementation (5 phases, 218 tests)
- Multi-platform packaging (Windows/macOS/Linux, x86_64/ARM64)
- CI/CD automation (GitHub Actions)
- Installation scripts (3 platforms)
- Comprehensive documentation (6,900+ lines)

### 📋 Planned

- **Phase 6 (Q4 2026)**: Keyboard & trackpad support
  - 100% keyboard navigation
  - Multi-touch gesture recognition
  - Vim/Emacs modes
  - Customizable shortcuts
  - [Full specification here](./KEYBOARD_TRACKPAD_SPEC.md)

- **App Store Submissions (Q4 2026)**: Windows Store, Mac App Store
- **Advanced Features (2027)**: Time travel, declarative web, transaction sandbox

---

## 📚 Documentation

**User Guides**
- [INSTALLATION.md](./INSTALLATION.md) - Platform-specific installation (1,200+ lines)
- [DISTRIBUTION.md](./DISTRIBUTION.md) - Distribution strategy (900+ lines)
- [Quick Start](./README.md) - This file

**Technical**
- [PLATFORM_SPECIFIC.md](./PLATFORM_SPECIFIC.md) - Architecture per platform (1,000+ lines)
- [KEYBOARD_TRACKPAD_SPEC.md](./KEYBOARD_TRACKPAD_SPEC.md) - Phase 6 specification (1,200+ lines)
- [PACKAGE_SUMMARY.md](./PACKAGE_SUMMARY.md) - Delivery overview (800+ lines)

**Developer**
- [build.rs](./build.rs) - Platform detection, version metadata
- [Cargo.toml](./Cargo.toml) - Dependencies, profiles, features
- [.github/workflows/](./github/workflows/) - CI/CD pipeline

---

## 🎯 Why Himalayas?

### Not "Chrome with AI"

Himalayas is ground-up agent-native. Every design decision prioritizes agents:
- Headless-first (GUI is a client)
- Runtime-first (processes optional)
- Permission-first (zero-trust)
- Audit-first (every action logged)
- Ephemeral-first (no persistence)

### Perfect For

- ✅ Government automation (DigiLocker, eSign, Aadhaar)
- ✅ Enterprise RPA (workflow automation)
- ✅ Privacy-conscious users (no tracking, no cookies)
- ✅ Developers (extensible APIs, headless mode)
- ✅ Researchers (transparent, auditable)
- ✅ Power users (keyboard-first, customizable)

### What It's Not

- ❌ Chrome replacement (different architecture)
- ❌ Privacy wrapper (built-in, not bolted-on)
- ❌ Extension of another browser (independent)
- ❌ Closed ecosystem (open APIs)

---

## 🤝 Contributing

We're actively building. Interested in:

- Security hardening?
- Performance optimization?
- Platform-specific features?
- Documentation?
- Testing?

See [CONTRIBUTING.md](./CONTRIBUTING.md) to get started.

---

## 📜 License

**Proprietary License** - Free to use with explicit attribution

All code, documentation, and designs in this repository are proprietary. You are free to use, modify, and distribute this software provided you give explicit attribution to the original author (Georgi Mammen Mullassery).

See [LICENSE](./LICENSE) for details.

---

## 🙋 Support

- **GitHub Issues**: [Bug reports & feature requests](https://github.com/Mullassery/Himalayas/issues)
- **GitHub Discussions**: [Questions & ideas](https://github.com/Mullassery/Himalayas/discussions)
- **Email**: mullassery@gmail.com

---

## 📈 Metrics

- **Code**: 218 tests (100% passing)
- **Architecture**: 22 design documents (80,000+ words)
- **Documentation**: 6,900+ lines across 6 files
- **Platforms**: 3 (Windows, macOS, Linux)
- **Architectures**: 2 per platform (x86_64, ARM64)
- **CI/CD**: Fully automated multi-platform builds

---

<div align="center">

**Himalayas Browser: Reaching the peak of autonomous computing.** 🏔️

Built with ❤️ by [Georgi Mammen Mullassery](https://github.com/Mullassery)

[⭐ Star us on GitHub!](https://github.com/Mullassery/Himalayas)

</div>
