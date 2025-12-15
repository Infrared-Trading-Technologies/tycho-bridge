# Contributing to Tycho Bridge

Thank you for your interest in contributing to Tycho Bridge! This document provides guidelines and information for contributors.

## Getting Started

1. **Fork the repository** and clone it locally
2. **Install prerequisites**:
   - Rust (nightly toolchain)
   - Protobuf compiler (`protoc`)
   - Docker (for integration tests)

3. **Set up your environment**:
   ```bash
   cp .env.example .env
   # Edit .env with your Tycho API credentials
   ```

4. **Build the project**:
   ```bash
   cargo build
   ```

5. **Run tests**:
   ```bash
   cargo test
   ```

## Development Workflow

### Code Style

- Follow Rust idioms and conventions
- Run `cargo fmt` before committing
- Run `cargo clippy` and address any warnings
- Write descriptive commit messages

### Testing

- **Unit tests**: Run with `cargo test`
- **Integration tests**: Require `TYCHO_API_KEY` environment variable
  ```bash
  TYCHO_API_KEY=<key> cargo test --test integration_tests -- --ignored
  ```

### Pull Requests

1. Create a feature branch from `main`
2. Make your changes with clear, atomic commits
3. Ensure all tests pass
4. Update documentation if needed
5. Open a pull request with a clear description

### Commit Messages

Use clear, descriptive commit messages:
- `feat:` for new features
- `fix:` for bug fixes
- `docs:` for documentation changes
- `refactor:` for code refactoring
- `test:` for test additions/modifications
- `chore:` for maintenance tasks

## Architecture Guidelines

This project follows these principles:

- **SOLID Principles** - Single responsibility, dependency inversion, etc.
- **12-Factor App** - Configuration via environment variables, stateless processes
- **No Mocking Tycho SDKs** - We use the official `tycho-simulation` and `tycho-execution` crates directly

## Reporting Issues

When reporting issues, please include:

- Rust version (`rustc --version`)
- Operating system
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs or error messages

## Questions?

Feel free to open an issue for questions or join the discussion in existing issues.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
