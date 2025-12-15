# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in Tycho Bridge, please report it responsibly.

### How to Report

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, please email security concerns to the maintainers privately. You can reach out via:

1. GitHub Security Advisories (preferred): Use the "Report a vulnerability" button in the Security tab
2. Direct contact through GitHub Issues marked as confidential

### What to Include

When reporting a vulnerability, please include:

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Any suggested fixes (if applicable)

### Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Resolution**: Depends on severity and complexity

### Scope

This security policy applies to:

- The Tycho Bridge gRPC service
- The Go client library (`clients/go/`)
- Configuration and deployment scripts

### Out of Scope

- The upstream Tycho Indexer (report to [Propeller Heads](https://github.com/propeller-heads/tycho-indexer))
- Third-party dependencies (report to respective maintainers)

## Security Best Practices

When deploying Tycho Bridge:

1. **Never commit API keys** - Use environment variables or secrets management
2. **Use TLS** - Enable TLS for gRPC connections in production
3. **Network isolation** - Run the service in a private network when possible
4. **Keep dependencies updated** - Regularly update Rust dependencies

## Acknowledgments

We appreciate security researchers who help keep Tycho Bridge secure. Contributors who report valid vulnerabilities will be acknowledged (with permission) in release notes.
