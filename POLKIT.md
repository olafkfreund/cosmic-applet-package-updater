# PolicyKit Integration

## Overview

The COSMIC Package Updater Applet now includes **PolicyKit (polkit) integration** for secure privilege escalation. This provides a more user-friendly and secure alternative to requiring passwordless sudo configuration.

## Benefits of PolicyKit

### Security Advantages
- **Fine-grained permission control**: PolicyKit allows specific actions to be authorized independently
- **User authentication dialogs**: Graphical password prompts instead of terminal-based sudo
- **Session-based caching**: Authorization persists for the session (configurable)
- **Audit logging**: All privileged operations are logged by PolicyKit
- **No sudoers modifications**: No need to edit `/etc/sudoers` or create sudoers.d files

### User Experience
- **Graphical authentication**: Native desktop authentication dialogs
- **Clear messaging**: Each action shows what it will do before asking for authentication
- **Smart caching**: Won't ask for password repeatedly within a session
- **Better security**: Reduces attack surface compared to blanket sudo permissions

## Architecture

### Components

1. **PolicyKit Policy File** (`policy/com.github.cosmic-ext.package-updater.policy`)
   - Defines authorized actions
   - Specifies who can perform actions
   - Configures authentication requirements

2. **Rust PolicyKit Module** (`src/polkit.rs`)
   - D-Bus communication with PolicyKit daemon
   - Authorization checking and requesting
   - Privilege escalation via pkexec
   - Automatic fallback to sudo if PolicyKit unavailable

3. **Integration in Package Manager** (`src/package_manager.rs`)
   - NixOS update checking uses PolicyKit when available
   - Graceful degradation to sudo for systems without PolicyKit
   - Clear error messages guide users to proper setup

### Defined Actions

| Action ID | Purpose | Default Policy |
|-----------|---------|----------------|
| `com.github.cosmic-ext.package-updater.check` | Check for updates | `auth_admin_keep` |
| `com.github.cosmic-ext.package-updater.update` | Install updates | `auth_admin_keep` |
| `com.github.cosmic-ext.package-updater.update-channels` | Update NixOS channels | `auth_admin_keep` |
| `com.github.cosmic-ext.package-updater.update-flakes` | Update NixOS flakes | `auth_admin_keep` |

### Policy Levels

- `auth_admin_keep`: Requires admin authentication, cached for session
- `auth_admin`: Requires admin authentication, not cached
- `yes`: Always allow (not recommended for privileged actions)
- `no`: Always deny

## Installation

### Automatic Installation

When installing via the justfile, the PolicyKit policy is automatically installed:

```bash
just build-release
sudo just install
```

This installs the policy file to:
```
/usr/share/polkit-1/actions/com.github.cosmic-ext.package-updater.policy
```

### Manual Installation

If you need to install the policy manually:

```bash
sudo install -Dm0644 \
    policy/com.github.cosmic-ext.package-updater.policy \
    /usr/share/polkit-1/actions/com.github.cosmic-ext.package-updater.policy
```

### NixOS Installation

For NixOS, the policy file should be included in the package derivation:

```nix
{
  # In your package derivation
  installPhase = ''
    mkdir -p $out/share/polkit-1/actions
    cp policy/com.github.cosmic-ext.package-updater.policy \
       $out/share/polkit-1/actions/
  '';
}
```

## Verification

### Check PolicyKit is Available

```bash
# Check if pkexec is installed
which pkexec

# Check if PolicyKit daemon is running
systemctl status polkit
```

### Test Authorization

```bash
# Test if you can get authorization (will show dialog)
pkexec echo "PolicyKit working"
```

### View Installed Actions

```bash
# List all PolicyKit actions
pkaction | grep cosmic-ext

# Show details for a specific action
pkaction --verbose --action-id com.github.cosmic-ext.package-updater.check
```

## Customization

### Changing Authorization Requirements

Edit the policy file at:
```
/usr/share/polkit-1/actions/com.github.cosmic-ext.package-updater.policy
```

Example: Allow updates without authentication for wheel group members:

```xml
<action id="com.github.cosmic-ext.package-updater.update">
  <description>Install package updates</description>
  <message>Authentication is required to install package updates</message>
  <defaults>
    <allow_any>auth_admin</allow_any>
    <allow_inactive>auth_admin</allow_inactive>
    <allow_active>yes</allow_active>  <!-- Changed from auth_admin_keep -->
  </defaults>
</action>
```

### Group-Based Rules

Create a custom rule file in `/etc/polkit-1/rules.d/`:

```javascript
// /etc/polkit-1/rules.d/50-cosmic-package-updater.rules
polkit.addRule(function(action, subject) {
    if (action.id == "com.github.cosmic-ext.package-updater.update" &&
        subject.isInGroup("wheel")) {
        return polkit.Result.YES;  // Allow without password
    }
});
```

### Session-Based Caching

Adjust cache timeout by modifying the policy:

```xml
<annotate key="org.freedesktop.policykit.imply">
  com.github.cosmic-ext.package-updater.check
  com.github.cosmic-ext.package-updater.update
</annotate>
```

## Fallback Behavior

The applet intelligently handles systems without PolicyKit:

1. **Check PolicyKit availability**
   - Looks for pkexec binary
   - Tests D-Bus connection to PolicyKit daemon

2. **If PolicyKit available**: Use pkexec for privilege escalation
3. **If PolicyKit unavailable**: Fall back to sudo with appropriate checks

### Fallback Error Messages

When PolicyKit is not available, users see helpful messages:

```
NixOS channels mode requires passwordless sudo or PolicyKit.

Option 1 (Recommended): PolicyKit is not available or failed.
Install PolicyKit and ensure pkexec is available.

Option 2: Configure passwordless sudo by adding to /etc/sudoers.d/nixos-rebuild:
%wheel ALL=(ALL) NOPASSWD: /run/current-system/sw/bin/nixos-rebuild
```

## Troubleshooting

### PolicyKit Authentication Fails

**Symptom**: Authentication dialog appears but always fails

**Solutions**:
1. Check PolicyKit daemon is running:
   ```bash
   systemctl status polkit
   ```

2. Check your user is in the appropriate group:
   ```bash
   groups
   # Should include 'wheel' or 'sudo'
   ```

3. Check PolicyKit logs:
   ```bash
   journalctl -u polkit -n 50
   ```

### No Authentication Dialog Appears

**Symptom**: Command fails immediately without showing dialog

**Solutions**:
1. Verify pkexec is installed:
   ```bash
   which pkexec
   ```

2. Check policy file is installed:
   ```bash
   ls -l /usr/share/polkit-1/actions/com.github.cosmic-ext.package-updater.policy
   ```

3. Test PolicyKit manually:
   ```bash
   pkexec echo test
   ```

### Permission Denied Errors

**Symptom**: "Authorization denied" or similar errors

**Solutions**:
1. Check the action is defined:
   ```bash
   pkaction --verbose --action-id com.github.cosmic-ext.package-updater.check
   ```

2. Verify your user has admin rights:
   ```bash
   id -Gn | grep -E 'wheel|sudo'
   ```

3. Try with explicit authentication:
   ```bash
   pkexec --user root true
   ```

## Security Considerations

### Advantages over Passwordless Sudo

| Feature | PolicyKit | Passwordless Sudo |
|---------|-----------|-------------------|
| Scope | Per-action | All commands |
| Audit | Full logging | Basic logging |
| Revocation | Immediate | Requires sudoers edit |
| User Experience | Graphical dialog | Terminal prompt |
| Security | Fine-grained | Broad permissions |

### Best Practices

1. **Don't modify default policies** unless necessary
2. **Use group-based rules** instead of per-user rules
3. **Monitor PolicyKit logs** for suspicious activity
4. **Keep PolicyKit updated** for security patches
5. **Test policy changes** before deploying to production

### Attack Surface Reduction

PolicyKit reduces attack surface compared to sudo:

- **No SETUID binaries**: pkexec uses capabilities, not SETUID
- **D-Bus authentication**: Uses secure D-Bus protocol
- **Session isolation**: Authorization limited to user session
- **Audit trail**: All operations logged to system journal

## Development

### Testing PolicyKit Integration

Run the integration tests:

```bash
cd package-updater
cargo test polkit
```

### Adding New Actions

1. Define action in policy file:
   ```xml
   <action id="com.github.cosmic-ext.package-updater.my-action">
     <description>My new action</description>
     <message>Authentication required for my action</message>
     <defaults>
       <allow_active>auth_admin_keep</allow_active>
     </defaults>
   </action>
   ```

2. Add constant in `src/polkit.rs`:
   ```rust
   pub const POLKIT_ACTION_MY_ACTION: &str = "com.github.cosmic-ext.package-updater.my-action";
   ```

3. Use in code:
   ```rust
   let output = polkit::execute_privileged(
       "my-command",
       &["arg1", "arg2"],
       polkit::POLKIT_ACTION_MY_ACTION,
       "My authentication message",
   ).await?;
   ```

## References

- [PolicyKit Documentation](https://www.freedesktop.org/software/polkit/docs/latest/)
- [PolicyKit Manual Pages](https://www.freedesktop.org/software/polkit/docs/latest/polkit.8.html)
- [pkexec Manual](https://www.freedesktop.org/software/polkit/docs/latest/pkexec.1.html)
- [Writing PolicyKit Policies](https://www.freedesktop.org/software/polkit/docs/latest/polkit-policy-file-validate.8.html)

## Support

For issues with PolicyKit integration:
1. Check this documentation first
2. Review the troubleshooting section
3. Check PolicyKit system logs
4. Open an issue on GitHub with:
   - Distribution and version
   - PolicyKit version (`pkexec --version`)
   - Relevant log output
   - Steps to reproduce

---

**Version**: 1.2.0
**Last Updated**: 2026-01-16
**Status**: Production Ready ✅
