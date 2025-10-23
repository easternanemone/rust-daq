# Feature Request: Whitelist Tailscale for Claude Code Sandbox

## Summary

Please whitelist Tailscale package sources to enable Claude Code sandboxes to connect to private Tailscale networks.

## Requested Whitelisting

Add the following domains to the Claude Code sandbox network allowlist:

- `tailscale.com` (install script)
- `pkgs.tailscale.com` (package repository)
- Tailscale coordination server endpoints (for establishing connections)

## Use Case

Enable secure connections from Claude Code sandboxes to centralized private services via Tailscale:

1. **Centralized databases** - Single source of truth across all development environments
2. **Centralized MCP servers** - Shared AI tools and services
3. **Private development infrastructure** - Databases, APIs, issue trackers (e.g., beads)
4. **CI/CD integration** - Consistent access patterns with GitHub Actions and other CI systems

### Example Architecture

```
┌─────────────────────────────────────────────────────────┐
│              User's Tailscale Private Network           │
│                                                         │
│  ┌──────────────────┐         ┌───────────────────┐   │
│  │ Central Server   │◄────────┤ Claude Code       │   │
│  │ (User's Machine) │Tailscale│ Sandbox           │   │
│  │                  │         │                   │   │
│  │ • MCP servers    │         │ • Tailscale       │   │
│  │ • Databases      │         │ • Connects to     │   │
│  │ • Issue tracker  │         │   private server  │   │
│  │ • Dev tools      │         │                   │   │
│  └──────────────────┘         └───────────────────┘   │
│           ▲                            ▲               │
│           │                            │               │
│  ┌────────┴────────┐         ┌────────┴────────┐     │
│  │ GitHub Actions  │         │ Other Envs      │     │
│  │ (CI/CD)         │         │ (Prod/Stage)    │     │
│  └─────────────────┘         └─────────────────┘     │
└─────────────────────────────────────────────────────────┘
```

## Current Status

All Tailscale installation methods return **403 Forbidden** in Claude Code sandbox:

### Attempted Installation Methods

1. **Official install script:**
   ```bash
   curl -fsSL https://tailscale.com/install.sh | sh
   # Error: curl: (22) The requested URL returned error: 403
   ```

2. **Package repository:**
   ```bash
   curl -fsSL https://pkgs.tailscale.com/stable/ubuntu/noble.noarmor.gpg
   # Error: curl: (22) The requested URL returned error: 403
   ```

3. **APT installation (after adding repository):**
   ```bash
   apt-get install tailscale
   # Error: 403 Forbidden on pkgs.tailscale.com
   ```

## Why This Matters

### 1. **Security**
- Eliminates need to expose services to public internet
- Uses WireGuard-based encrypted tunnels
- Zero-trust network access
- Better than VPNs or public endpoints

### 2. **Developer Experience**
- Single source of truth (no git sync conflicts)
- Real-time updates across all environments
- Consistent development/production parity
- Works identically in local dev, Claude Code, and CI/CD

### 3. **Real-World Use Cases**

**Issue Tracking (beads):**
- Central beads database on user's server
- All environments (Claude Code, GitHub Actions, local) connect via Tailscale
- Real-time issue updates, no merge conflicts
- See: https://github.com/steveyegge/beads

**MCP Servers:**
- Host MCP servers on user's infrastructure
- All Claude Code sessions connect to same servers
- Centralized configuration and state management

**Private Databases:**
- Development databases on private network
- Claude Code can run integration tests
- No public exposure or complex firewall rules

### 4. **Existing Ecosystem Support**

Tailscale already has first-class support in:
- ✅ GitHub Actions: https://github.com/tailscale/github-action
- ✅ GitLab CI: https://tailscale.com/kb/1207/gitlab-ci
- ✅ CircleCI, Jenkins, etc.

Claude Code would join this ecosystem.

## Benefits of Whitelisting

1. **Zero public exposure** - Services stay on private network
2. **Better security** - WireGuard encryption, ACLs, audit logs
3. **Easier setup** - No complex firewall rules or reverse proxies
4. **Consistent access** - Same pattern across all environments
5. **NAT traversal** - Works behind firewalls and NATs
6. **Cross-platform** - Works on all OS platforms

## Alternative Considered (Why It's Insufficient)

**Git-based sync (current workaround):**
- ❌ Merge conflicts with concurrent edits
- ❌ Delayed sync (manual git push/pull)
- ❌ No real-time updates
- ❌ Doesn't work for databases or stateful services
- ❌ Complex for multi-environment workflows

## Comparison to Current Whitelist

The Claude Code sandbox already allows:
- ✅ PyPI packages (`pip install` works)
- ✅ Ubuntu packages (`apt-get` works)
- ✅ GitHub repositories (git clone works)

**Tailscale fits the same pattern** - it's a legitimate developer tool that enhances security and enables private network access without compromising sandbox safety.

## Security Considerations

**Tailscale does NOT bypass sandbox security:**
- User must authenticate to their own Tailnet (requires auth key)
- Only grants access to user's private network, not internet
- User controls ACLs and access policies
- Anthropic can audit Tailscale traffic if needed
- Still sandboxed from host system

**This is MORE secure than alternatives:**
- Better than exposing services to public internet
- Better than complex port forwarding
- Better than VPNs that grant broad network access
- Zero-trust model with explicit ACLs

## Implementation Suggestion

Minimal changes required:

1. **Whitelist domains:**
   - `tailscale.com`
   - `pkgs.tailscale.com`
   - Tailscale DERP servers (coordination)

2. **No code changes needed** - Standard Ubuntu package installation

3. **User opt-in** - Only users who install Tailscale use it

## Technical Details

**Package source:**
```bash
deb [signed-by=/usr/share/keyrings/tailscale-archive-keyring.gpg] \
  https://pkgs.tailscale.com/stable/ubuntu noble main
```

**Install size:** ~50MB

**Dependencies:** Standard Ubuntu packages (already in sandbox)

## Community Impact

This would enable an entire class of use cases:

- **Remote development** - Access to company infrastructure
- **Team collaboration** - Shared services and databases
- **Hybrid workflows** - Local + cloud development
- **Enterprise adoption** - Private network requirements

## References

- **Tailscale Documentation:** https://tailscale.com/kb/
- **Tailscale GitHub Action:** https://github.com/tailscale/github-action
- **Claude Code Sandboxing:** https://docs.claude.com/en/docs/claude-code/sandboxing
- **Example Project:** rust-daq beads integration (see TAILSCALE_INTEGRATION.md)

## Proposed Timeline

- **Phase 1:** Whitelist domains (minimal risk, high value)
- **Phase 2:** Gather user feedback
- **Phase 3:** Consider additional Tailscale features if needed

## Call to Action

Please consider whitelisting Tailscale to enable secure private network access for Claude Code users. This aligns with developer needs for centralized, secure infrastructure access.

---

**Submitted by:** rust-daq project maintainer
**Date:** 2025-10-23
**Related:** Centralized beads database, MCP server architecture
