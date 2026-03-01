# Contributing to AegisGate

Thank you for your interest in contributing to AegisGate. This document outlines the process for contributing to the project.

## License and Copyright

By contributing code to the AegisGate project, you agree to license your contributions under the Apache License 2.0. When you submit any copyrighted material via pull request or other means, you agree to license the material under the project's open source license and warrant that you have the legal authority to do so.

When changing existing source code, you do not alter the copyright of the original file(s). The copyright remains with the original creator(s).

## Getting Started

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```
   git clone https://github.com/YOUR_USERNAME/aegisgate.git
   cd aegisgate
   ```
3. Create a branch for your changes:
   ```
   git checkout -b feature/your-feature-name
   ```

## Development Setup

### Prerequisites

- Rust 1.75 or later
- Docker and Docker Compose (for testing)

### Building

```
cargo build --manifest-path crates/aegis-proxy/Cargo.toml
```

### Running Tests

```
cargo test --workspace
```

### Running Locally

```
cargo run --manifest-path crates/aegis-proxy/Cargo.toml
```

## Making Changes

### Code Style

All code must follow Rust standard formatting and pass clippy checks:

```
cargo fmt --all
cargo clippy --all -- -D warnings
```

### Testing

- Write tests for new features and bug fixes
- Ensure all tests pass before submitting a pull request
- Include both unit tests and integration tests where applicable

### Documentation

- Update documentation for user-facing changes
- Update configuration examples in `config/` if adding new options
- Update README.md for significant features

## Submitting Changes

### Commit Message Format

Each commit message should follow this format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Type** must be one of:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, no logic change)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Maintenance tasks, dependency updates
- `ci`: CI/CD pipeline changes

**Scope** (optional): The component affected (e.g., `proxy`, `mqtt`, `rate-limit`, `metrics`)

**Subject**: Short description (imperative mood, lowercase, no period)

**Body** (optional): Detailed explanation of what and why (not how)

**Footer** (optional): Reference issues, breaking changes

**Example:**

```
feat(rate-limit): add per-connection token bucket

Implement token bucket algorithm for per-connection rate limiting
in addition to the existing per-IP limits. This provides more
granular control over connection rates.

Fixes #123
```

**Commit Message Guidelines:**
- Use imperative mood: "add feature" not "added feature"
- Keep subject line under 72 characters
- Reference issues with `Fixes #123` or `Closes #456`
- Use `BREAKING CHANGE:` in footer for breaking changes

### Developer Certificate of Origin

This project uses the Developer Certificate of Origin (DCO). All commits must be signed off to certify that you have the right to submit the code under the project's license.

Sign your commits using the `-s` flag:

```
git commit -s -m "your commit message"
```

This adds a `Signed-off-by` line to your commit message:

```
Signed-off-by: Your Name <your.email@example.com>
```

By signing off, you certify that you wrote the code or have the right to submit it under the Apache License 2.0.

### Pull Request Process

1. Update documentation and tests as needed
2. Ensure all tests pass and code passes `cargo fmt` and `cargo clippy`
3. Sign off on all commits (DCO requirement)
4. Push your branch to your fork
5. Open a pull request against the `main` branch
6. Provide a clear description of the changes and the problem they solve
7. Link to any related issues

### Pull Request Checklist

- [ ] Code builds without errors
- [ ] All tests pass
- [ ] Code is formatted with `cargo fmt`
- [ ] No clippy warnings
- [ ] Documentation updated
- [ ] Commits are signed off (DCO)

## Reporting Issues

### Bug Reports

When reporting bugs, include:

- Steps to reproduce the issue
- Expected behavior
- Actual behavior
- AegisGate version or commit hash
- Configuration file (redact sensitive information)
- Relevant logs and error messages

### Feature Requests

When requesting features, include:

- Use case and motivation
- Proposed solution or API
- Alternative solutions considered
- Impact on existing functionality

### Security Issues

Do not report security vulnerabilities through public GitHub issues. Instead, report them through GitHub Security Advisories or contact the maintainers directly.

## Code Review

All submissions require review before being merged. Reviewers will check for:

- Correctness and code quality
- Test coverage
- Documentation completeness
- Performance implications
- Security considerations
- Adherence to project conventions

Be responsive to feedback and keep your pull request up to date with the `main` branch.

## Community

- Be respectful and constructive
- Follow the [Code of Conduct](CODE_OF_CONDUCT.md)
- Help others when you can
- Focus on what is best for the project and community

## License

By contributing to AegisGate, you agree that your contributions will be licensed under the Apache License 2.0.