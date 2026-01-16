# Security Policy

## Overview

This document outlines the security measures, best practices, and vulnerability disclosure policy for the COSMIC Package Updater Applet.

---

## Security Features

### 🔒 Implemented Protections (v1.2.0+)

#### 1. Command Injection Prevention
- **Threat**: Malicious paths in environment variables could lead to arbitrary command execution
- **Mitigation**: All user-influenced paths are escaped using the `shell-escape` crate before shell execution
- **Implementation**: `package-updater/src/app.rs:356`
- **Testing**: Unit tests verify escaping of special characters

#### 2. Atomic File Locking
- **Threat**: Race conditions between multiple applet instances could corrupt state or duplicate work
- **Mitigation**: Uses Unix `flock` system call for atomic exclusive locks
- **Implementation**: `package-updater/src/package_manager.rs:241-268`
- **Details**: Non-blocking lock acquisition with proper error handling
- **Testing**: 7 integration tests verify lock behavior under various scenarios

#### 3. PolicyKit Privilege Escalation
- **Threat**: Broad sudo permissions create large attack surface
- **Mitigation**: PolicyKit provides fine-grained, action-specific privilege escalation with user authentication
- **Implementation**: `package-updater/src/polkit.rs`
- **Benefits**:
  - Per-action permissions (check updates, install updates, etc.)
  - Graphical authentication dialogs with clear action descriptions
  - Session-based authorization caching
  - Complete audit trail in system logs
  - Automatic fallback to sudo when PolicyKit unavailable
- **Policy File**: `/usr/share/polkit-1/actions/com.github.cosmic-ext.package-updater.policy`
- **Documentation**: See `POLKIT.md` for complete guide

#### 4. Privileged Operation Safety (Sudo Fallback)
- **Threat**: Hanging or blocking when sudo password is required
- **Mitigation**: Pre-flight check for passwordless sudo before attempting privileged operations (when PolicyKit unavailable)
- **Implementation**: `package-updater/src/package_manager.rs:682-689`
- **User Experience**: Clear error messages guide users to configure PolicyKit (recommended) or sudo (fallback)

#### 5. Executable Path Validation
- **Threat**: Execution of malicious binaries from unsafe locations (e.g., `/tmp`, user home)
- **Mitigation**: Only executables in system directories are trusted
- **Allowed Paths**:
  - `/usr/`
  - `/bin/`
  - `/sbin/`
  - `/nix/store/`
  - `/run/current-system/`
  - `/opt/`
- **Implementation**: `package-updater/src/package_manager.rs:170-195`

---

## Threat Model

### In Scope

1. **Command Injection**: Malicious input via environment variables or configuration
2. **Race Conditions**: Concurrent access to shared resources
3. **Privilege Escalation**: Misuse of sudo or other privileged operations
4. **Path Traversal**: Execution of binaries outside trusted locations
5. **Information Disclosure**: Leaking sensitive system information

### Out of Scope

1. **Physical Access**: Attacks requiring physical access to the machine
2. **Root Compromise**: If attacker has root, game over (by design)
3. **Kernel Vulnerabilities**: OS-level security issues
4. **Supply Chain**: Compromised dependencies (use tools like `cargo-audit`)

---

## Security Best Practices for Users

### For NixOS Users (Channels Mode)

**Configure Passwordless Sudo for nixos-rebuild:**

```bash
# Create sudoers file (as root)
sudo visudo -f /etc/sudoers.d/nixos-rebuild

# Add this line (replace %wheel with your group):
%wheel ALL=(ALL) NOPASSWD: /run/current-system/sw/bin/nixos-rebuild

# Set proper permissions
sudo chmod 0440 /etc/sudoers.d/nixos-rebuild
```

**Why**: Prevents the applet from hanging while waiting for password input.

**Security Considerations**:
- Only grants permission for `nixos-rebuild`, not all commands
- Uses full path to prevent PATH manipulation
- Members of `wheel` group only (adjust for your system)

### For All Users

1. **Keep the applet updated**: Security fixes are released regularly
2. **Review permissions**: Understand what package managers do with root access
3. **Monitor logs**: Check system logs for unexpected behavior
4. **Use system package managers only**: Don't configure custom/untrusted package managers
5. **Validate configurations**: Ensure NixOS config paths point to trusted locations

---

## Security Audit Results

### Version 1.2.0 (2026-01-16)

**Security Enhancements**:

| ID | Type | Enhancement | Status |
|----|------|-------------|--------|
| SEC-005 | **HIGH** | PolicyKit integration for privilege escalation | ✅ Implemented |
| SEC-006 | **MEDIUM** | Integration tests for lock mechanism | ✅ Implemented |

**PolicyKit Security Benefits**:
- Fine-grained permission control per action
- Graphical authentication dialogs with clear messaging
- Session-based authorization caching
- Complete audit trail in system logs
- Reduces attack surface compared to passwordless sudo
- Automatic fallback to sudo when unavailable

**Testing Improvements**:
- Added 7 integration tests for file locking
- Async test coverage for concurrent access scenarios
- Verification of lock retry logic
- Sync file notification testing

### Version 1.1.0 (2026-01-16)

**Vulnerabilities Fixed**:

| ID | Severity | Issue | Status |
|----|----------|-------|--------|
| SEC-001 | **HIGH** | Command injection via marker file paths | ✅ Fixed |
| SEC-002 | **MEDIUM** | Race condition in lock acquisition | ✅ Fixed |
| SEC-003 | **MEDIUM** | Sudo command hangs without passwordless config | ✅ Fixed |
| SEC-004 | **MEDIUM** | No validation of executable paths | ✅ Fixed |

**Static Analysis Tools Used**:
- `cargo clippy --all-features -- -W clippy::pedantic`
- Manual code review
- Security-focused code review by AI assistant
- Integration testing for concurrency scenarios

---

## Reporting a Vulnerability

### Process

1. **Do NOT** open a public GitHub issue for security vulnerabilities
2. Email the maintainer directly (check `Cargo.toml` for contact info)
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)
4. Allow 90 days for a fix before public disclosure

### What to Expect

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 7 days
- **Fix Timeline**: Depends on severity
  - Critical: 7-14 days
  - High: 14-30 days
  - Medium: 30-60 days
  - Low: 60-90 days
- **Credit**: You will be credited in the release notes (unless you prefer anonymity)

### Hall of Fame

Security researchers who have responsibly disclosed vulnerabilities:

- *Be the first!*

---

## Secure Development Practices

### Code Review Requirements

All code changes undergo:
1. Automated linting with `clippy`
2. Security-focused manual review
3. Test coverage for security-critical paths

### Dependency Management

- Regular `cargo audit` runs
- Minimal dependency footprint
- Trusted dependencies only (crates.io, GitHub)
- No binary dependencies from untrusted sources

### Build Security

- Reproducible builds via Nix flake
- No network access during build (Nix sandbox)
- Verified checksums for all dependencies
- Builds on NixOS use isolated build environment

---

## Security Checklist for Contributions

When submitting code, ensure:

- [ ] No use of `.unwrap()` on user-influenced data
- [ ] All file operations have proper error handling
- [ ] Shell commands properly escape all arguments
- [ ] Privileged operations have pre-flight checks
- [ ] Executables validated before execution
- [ ] Race conditions considered for shared resources
- [ ] Unit tests cover security-critical code paths
- [ ] Documentation updated for security-relevant changes

---

## Additional Security Resources

### Related Documentation

- **CHANGES.md**: Detailed security fix history
- **CLAUDE.md**: Security patterns and best practices
- **Cargo.toml**: Dependency versions and features

### External Resources

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [cargo-audit](https://github.com/RustSec/rustsec/tree/main/cargo-audit)
- [Secure Coding in Rust](https://www.rust-lang.org/security)

---

## License

This security policy is licensed under the same terms as the project (GPL-3.0).

---

## Contact

For non-security issues, use GitHub Issues.
For security concerns, email the maintainers directly.

---

**Last Updated**: 2026-01-16
**Version**: 1.2.0
