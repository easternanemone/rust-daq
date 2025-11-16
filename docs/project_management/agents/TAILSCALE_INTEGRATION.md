# Tailscale Integration for Centralized Beads Access

## Architecture Overview

This document describes how to use Tailscale to connect Claude Code sandboxes and other environments to a centralized beads server and other services.

## Intended Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Your Tailscale Network                   │
│                                                             │
│  ┌──────────────────┐         ┌──────────────────┐        │
│  │ Central Server   │         │ Claude Code      │        │
│  │ (Your Machine)   │◄────────┤ Sandbox          │        │
│  │                  │ Tailscale│                  │        │
│  │ - bd daemon      │         │ - Tailscale      │        │
│  │ - beads DB       │         │ - bd client      │        │
│  │ - MCP servers    │         │ - connects to    │        │
│  │ - Other services │         │   your server    │        │
│  └──────────────────┘         └──────────────────┘        │
│           │                            │                    │
│           │                            │                    │
│  ┌────────┴─────────┐         ┌────────┴─────────┐        │
│  │ GitHub Actions   │         │ Other Envs       │        │
│  │ - Tailscale      │         │ - Tailscale      │        │
│  │ - bd client      │         │ - bd client      │        │
│  └──────────────────┘         └──────────────────┘        │
└─────────────────────────────────────────────────────────────┘
```

**Benefits:**
- ✅ Single source of truth for beads database
- ✅ No git sync conflicts
- ✅ Real-time issue updates across all environments
- ✅ Centralized MCP servers and services
- ✅ Works with GitHub Actions, CI/CD, Claude Code, etc.

## Current Status in Claude Code Sandbox

### Tailscale Installation Attempts (All Failed)

We tried multiple installation methods, all blocked by sandbox restrictions:

1. **Official install script:**
   ```bash
   curl -fsSL https://tailscale.com/install.sh | sh
   # Result: 403 Forbidden
   ```

2. **Direct package repository:**
   ```bash
   curl -fsSL https://pkgs.tailscale.com/stable/ubuntu/noble.noarmor.gpg
   # Result: 403 Forbidden
   ```

3. **APT repository (after adding sources):**
   ```bash
   apt-get install tailscale
   # Result: 403 Forbidden on pkgs.tailscale.com
   ```

### Root Cause

Claude Code sandbox blocks access to:
- `tailscale.com` (install script)
- `pkgs.tailscale.com` (package repository)
- Various other third-party package sources

This is by design for sandbox security, but **should be whitelisted** for Tailscale to enable this architecture.

## Expected Setup (When Tailscale is Whitelisted)

### Step 1: Central Server Setup

On your central server (the one always running on your Tailnet):

```bash
# Install Tailscale (already done on your machine)
curl -fsSL https://tailscale.com/install.sh | sh
sudo tailscale up

# Install bd CLI
curl -fsSL https://raw.githubusercontent.com/steveyegge/beads/main/scripts/install.sh | bash

# Initialize beads database in a persistent location
mkdir -p ~/beads-central
cd ~/beads-central
bd init --prefix daq

# Start bd daemon in global mode with auto-commit
bd daemon --global --auto-commit --auto-push

# Make sure it starts on boot
cat > /etc/systemd/system/beads-daemon.service <<EOF
[Unit]
Description=Beads Global Daemon
After=network.target tailscaled.service

[Service]
Type=simple
User=$USER
ExecStart=$(which bd) daemon --global --auto-commit --auto-push
Restart=always
Environment=BEADS_DB=/home/$USER/beads-central/.beads/daq.db

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl enable beads-daemon.service
sudo systemctl start beads-daemon.service
```

### Step 2: Claude Code Sandbox Setup (When Tailscale Works)

In Claude Code sandbox:

```bash
# Install Tailscale
curl -fsSL https://tailscale.com/install.sh | sh

# Connect to your Tailnet (requires auth)
# You'll need to use a reusable auth key or one-time URL
sudo tailscale up --authkey tskey-auth-XXXXXXXXXX

# Install bd CLI
curl -fsSL https://raw.githubusercontent.com/steveyegge/beads/main/scripts/install.sh | bash

# Configure to use your central server
export BEADS_DB=your-server-ip:50051  # bd daemon RPC endpoint
# or via Tailscale hostname
export BEADS_DB=your-central-server.your-tailnet.ts.net:50051

# Test connection
bd list
bd ready
```

### Step 3: GitHub Actions Setup

In your `.github/workflows/`:

```yaml
name: CI with Beads
on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Connect to Tailscale
        uses: tailscale/github-action@v2
        with:
          oauth-client-id: ${{ secrets.TS_OAUTH_CLIENT_ID }}
          oauth-secret: ${{ secrets.TS_OAUTH_SECRET }}
          tags: tag:ci

      - name: Install bd CLI
        run: |
          curl -fsSL https://raw.githubusercontent.com/steveyegge/beads/main/scripts/install.sh | bash

      - name: Configure beads to use central server
        env:
          BEADS_SERVER: ${{ secrets.BEADS_SERVER_TAILSCALE }}
        run: |
          echo "BEADS_DB=$BEADS_SERVER:50051" >> $GITHUB_ENV

      - name: Update issue status
        run: |
          bd update $ISSUE_ID --status in_progress

      # ... rest of your CI steps ...

      - name: Close issue on success
        if: success()
        run: |
          bd close $ISSUE_ID --reason "CI passed"
```

## Requesting Tailscale Whitelisting

To enable this architecture, you should request Tailscale access from Anthropic:

### Contact Information

1. **Claude Code Support:**
   - https://github.com/anthropics/claude-code/issues
   - Describe your use case (centralized services via Tailscale)

2. **Feature Request:**
   ```
   Title: Whitelist Tailscale for Claude Code Sandbox

   Description:
   Request to whitelist Tailscale package sources for Claude Code sandboxes:
   - tailscale.com
   - pkgs.tailscale.com

   Use Case:
   Enable secure connections to centralized services (databases, MCP servers,
   beads issue tracker, etc.) via private Tailscale network. This enables:

   1. Centralized beads database accessible from all environments
   2. Centralized MCP servers
   3. Private services without exposing to public internet
   4. Works with GitHub Actions, CI/CD, and other environments

   Current Status:
   All Tailscale installation methods return 403 Forbidden in sandbox.

   Expected Behavior:
   Tailscale should be installable and usable for connecting to private tailnets.
   ```

## Alternative: Tailscale Subnet Router

If direct Tailscale in sandbox isn't possible, you could set up a subnet router:

### On Your Central Server

```bash
# Enable IP forwarding
echo 'net.ipv4.ip_forward = 1' | sudo tee -a /etc/sysctl.conf
sudo sysctl -p

# Start Tailscale as subnet router
sudo tailscale up --advertise-routes=10.0.0.0/24 --accept-routes

# Approve subnet routes in Tailscale admin console
```

### In Sandbox (If Routes Are Accessible)

```bash
# Try connecting directly to Tailscale IP
curl http://100.x.x.x:50051/health  # Your server's Tailscale IP

# If this works, you can use bd remotely without installing Tailscale
export BEADS_DB=100.x.x.x:50051
bd list
```

## Current Workaround (Git-Based Sync)

Until Tailscale is whitelisted, use the git-based approach:

**Pros:**
- ✅ Works now in Claude Code sandbox
- ✅ No network dependencies
- ✅ Audit trail via git history

**Cons:**
- ❌ Potential merge conflicts
- ❌ Delayed sync (manual git push/pull)
- ❌ Not real-time

See [BEADS_INSTALLATION.md](BEADS_INSTALLATION.md) for details.

## Technical Details

### Why This Architecture Makes Sense

1. **Single Database:** No sync conflicts between environments
2. **Real-time Updates:** Changes visible immediately across all connected clients
3. **Centralized MCP:** One set of MCP servers for all environments
4. **Secure:** Private Tailscale network, no public exposure
5. **Scalable:** Add new environments easily (just install Tailscale + bd)

### bd Daemon RPC Protocol

The bd daemon supports remote connections:

```bash
# On server
bd daemon --global --listen 0.0.0.0:50051

# From client
bd --server your-server:50051 list
bd --server your-server:50051 create "Remote issue"

# Or via environment variable
export BEADS_SERVER=your-server:50051
bd list
```

### Security Considerations

- Use Tailscale ACLs to restrict which nodes can access bd daemon
- Consider mTLS for additional security
- Rotate Tailscale auth keys regularly
- Use ephemeral auth keys for CI/CD environments

## Resources

- **Tailscale Documentation:** https://tailscale.com/kb/
- **Tailscale GitHub Action:** https://github.com/tailscale/github-action
- **beads Daemon Mode:** See beads README.md daemon section
- **Claude Code Sandboxing:** https://docs.claude.com/en/docs/claude-code/sandboxing

## Status Summary

**Current:**
- ❌ Tailscale blocked in Claude Code sandbox (403 Forbidden)
- ✅ beads-mcp MCP server installed and working
- ✅ Git-based sync works as fallback

**Requested:**
- 🔄 Whitelist Tailscale in Claude Code sandbox
- 🔄 Enable private network access for centralized services

**Expected After Whitelisting:**
- ✅ Real-time beads database access across all environments
- ✅ Centralized MCP servers
- ✅ No git sync conflicts
- ✅ Better integration with CI/CD pipelines
