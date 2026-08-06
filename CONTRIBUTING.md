# Contributing to Himalayas Browser

Thank you for your interest in contributing to Himalayas Browser!

## Getting Started

### Prerequisites

- Rust 1.75 or later
- Cargo
- Git

### Setup

```bash
# Clone the repository
git clone https://github.com/Mullassery/Himalayas-Browser.git
cd Himalayas-Browser

# Build
cargo build --release

# Run tests
cargo test --lib

# Run benchmarks
cargo benchmark
```

## Development Workflow

### Making Changes

1. **Create a branch** for your feature/fix
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following Rust best practices
   - Keep commits focused and atomic
   - Write descriptive commit messages
   - Add tests for new functionality

3. **Test thoroughly**
   ```bash
   cargo test --lib
   cargo build --release
   ```

4. **Push and create a PR**
   ```bash
   git push origin feature/your-feature-name
   ```

## Code Standards

### Rust Style

- Follow standard Rust naming conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Add doc comments for public APIs
- Write tests for all new functionality

### Testing

- Add unit tests in the same file as the code
- All tests must pass before submitting PR
- Target: >80% coverage on new code

### Documentation

- Update relevant `.md` files
- Add comments for non-obvious logic
- Keep documentation up-to-date with code changes

## Pull Request Process

1. **Describe your changes** clearly in the PR description
2. **Reference any related issues** (e.g., "Fixes #123")
3. **Ensure all tests pass** (`cargo test --lib`)
4. **Keep PRs focused** - one feature/fix per PR
5. **Respond to reviews** promptly

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Enhancement
- [ ] Documentation
- [ ] Refactoring

## Testing
Describe testing performed

## Checklist
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] All tests passing
- [ ] No breaking changes
```

## Issues

### Reporting Bugs

Use the [Bug Report](https://github.com/Mullassery/Himalayas-Browser/issues/new?template=bug_report.md) template.

**Include**:
- Clear reproduction steps
- Expected vs actual behavior
- Environment details (OS, version, architecture)
- Relevant logs or error messages

### Feature Requests

Use the [Feature Request](https://github.com/Mullassery/Himalayas-Browser/issues/new?template=feature_request.md) template.

**Include**:
- Clear description of the feature
- Problem it solves
- Proposed implementation
- Priority level

## Code Review

All submissions require review. We look for:

✅ **Correctness** - Does it work correctly?  
✅ **Quality** - Is it well-written and maintainable?  
✅ **Testing** - Are all cases covered?  
✅ **Documentation** - Is it documented?  
✅ **Performance** - Does it meet performance goals?  

## Architecture

Himalayas is organized into modules:

```
src/
├── lib.rs              # Main library exports
├── browser/            # Core browser functionality
├── daemon/             # Daemon process
├── device/             # Device integration (GNSS, sensors)
├── document/           # Document processing & AI
├── intelligence/       # Adaptive intelligence engine
├── ui/                 # UI system (context menus, adaptive)
├── permission/         # Permission management
└── health/             # Health monitoring
```

Each module is self-contained with its own tests and documentation.

## Build Targets

```bash
# Default (daemon)
cargo build --release

# With tests
cargo test --lib

# Benchmarks
cargo benchmark

# Cross-compile
cross build --release --target aarch64-unknown-linux-gnu
```

## Continuous Integration

All PRs automatically run:
- Tests (`cargo test --lib`)
- Builds for all platforms
- Code quality checks

## Questions or Need Help?

- **GitHub Issues**: https://github.com/Mullassery/Himalayas-Browser/issues
- **GitHub Discussions**: https://github.com/Mullassery/Himalayas-Browser/discussions
- **Email**: mullassery@gmail.com

## License

By contributing, you agree that your contributions will be licensed under the Proprietary License (Free to use with explicit attribution).

---

**Thank you for contributing to Himalayas Browser!** 🏔️
