# Multi-Environment Tailscale Access Guide

Complete guide for giving cloud environments (GitHub Actions, Claude Code, Jules, etc.) access to your Tailscale network for centralized services.

## Overview

**Goal:** Allow ephemeral cloud environments to securely access your private Tailscale network for centralized services (beads database, MCP servers, development databases, etc.)

## Part 1: Your Tailscale Setup (Do This First)

### 1.1 Create Reusable Auth Keys

Auth keys allow automated environments to join your Tailnet without manual approval.

**In Tailscale Admin Console** (https://login.tailscale.com/admin/settings/keys):

1. Go to **Settings** → **Keys**
2. Click **Generate auth key**
3. Configure for cloud environments:

```
Key type: Reusable
Description: "Cloud Environments (GitHub, Claude, Jules)"
Tags: tag:cloud-env
Expiration: 90 days (or longer)
Ephemeral: ✓ ENABLE THIS
  └─ Nodes disappear when disconnected (perfect for CI/CD)
Preauthorized: ✓ ENABLE THIS
  └─ Skip manual approval step
```

4. **Save the key** - you'll need it for each environment
   - Format: `tskey-auth-XXXXXXXXXX-YYYYYYYYYYYYYYYYYYYY`

### 1.2 Configure ACLs (Access Control)

**In Tailscale Admin Console** → **Access Controls**:

```json
{
  "tagOwners": {
    "tag:cloud-env": ["your-email@example.com"]
  },

  "acls": [
    // Allow cloud environments to access your central server
    {
      "action": "accept",
      "src": ["tag:cloud-env"],
      "dst": ["tag:server:*"]
    },

    // Allow cloud environments to access specific services
    {
      "action": "accept",
      "src": ["tag:cloud-env"],
      "dst": [
        "your-server:50051",  // beads daemon RPC
        "your-server:8080",   // MCP server
        "your-server:5432"    // PostgreSQL (example)
      ]
    },

    // Deny everything else by default
    {
      "action": "accept",
      "src": ["tag:cloud-env"],
      "dst": ["!*:*"]
    }
  ]
}
```

**Key Points:**
- Use tags to group ephemeral nodes
- Restrict access to only necessary ports
- Tag your central server as `tag:server`

### 1.3 Tag Your Central Server

On your always-on server:

```bash
# Apply server tag
sudo tailscale up --advertise-tags=tag:server

# Verify
tailscale status
```

### 1.4 Start Services on Central Server

```bash
# Install beads
curl -fsSL https://raw.githubusercontent.com/steveyegge/beads/main/scripts/install.sh | bash

# Initialize database
mkdir -p ~/beads-central
cd ~/beads-central
bd init --prefix daq

# Start daemon listening on Tailscale interface
# Get your Tailscale IP: tailscale ip -4
TAILSCALE_IP=$(tailscale ip -4)
bd daemon --global --listen $TAILSCALE_IP:50051

# Or listen on all interfaces (less secure)
bd daemon --global --listen 0.0.0.0:50051
```

## Part 2: Environment-Specific Setup

### 2.1 GitHub Actions (✅ WORKS NOW)

Tailscale has official support for GitHub Actions.

**In your workflow file** (`.github/workflows/ci.yml`):

```yaml
name: CI with Tailscale Access

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      # Connect to Tailscale
      - name: Connect to Tailscale
        uses: tailscale/github-action@v2
        with:
          oauth-client-id: ${{ secrets.TS_OAUTH_CLIENT_ID }}
          oauth-secret: ${{ secrets.TS_OAUTH_SECRET }}
          tags: tag:cloud-env

      # Now you have access to your Tailnet!
      - name: Install bd CLI
        run: |
          curl -fsSL https://raw.githubusercontent.com/steveyegge/beads/main/scripts/install.sh | bash

      - name: Use centralized beads
        env:
          BEADS_SERVER: ${{ secrets.TAILSCALE_SERVER_IP }}
        run: |
          # Connect to your central beads server
          bd --server $BEADS_SERVER:50051 list
          bd --server $BEADS_SERVER:50051 ready

          # Or set environment variable
          export BD_SERVER=$BEADS_SERVER:50051
          bd create "CI run started" -t task -p 2

      # Your build/test steps here
      - name: Run tests
        run: cargo test

      - name: Update beads on success
        if: success()
        run: bd close $ISSUE_ID --reason "CI passed"
```

**Setup GitHub Secrets:**

1. Create OAuth client in Tailscale: https://login.tailscale.com/admin/settings/oauth
2. Add to GitHub repo secrets:
   - `TS_OAUTH_CLIENT_ID`
   - `TS_OAUTH_SECRET`
   - `TAILSCALE_SERVER_IP` (your server's Tailscale IP)

**Documentation:** https://github.com/tailscale/github-action

### 2.2 Claude Code (❌ CURRENTLY BLOCKED)

**Current Status:** All Tailscale installation methods blocked (403 Forbidden)

**What You Can Do:**

#### Option A: Request Whitelisting (Recommended)

Submit feature request to Anthropic:
- **URL:** https://github.com/anthropics/claude-code/issues/new
- **Use template:** `.github/TAILSCALE_FEATURE_REQUEST.md` (already created)
- **Impact:** Would enable this for ALL users

#### Option B: Workaround (If You Have Local Install)

If you can install Tailscale on your local machine where you run Claude Code Desktop:

```bash
# On your local machine (outside sandbox)
sudo tailscale up --authkey tskey-auth-XXXXXXXXXX

# Then the sandbox might inherit network access
# (Untested - depends on Claude Code Desktop architecture)
```

#### Option C: Wait for Support

Use git-based sync until Tailscale is whitelisted.

### 2.3 Jules (Need Information)

**I need more details about Jules:**
- What is Jules? (Cloud IDE? CI/CD platform? Other?)
- What's the URL/documentation?
- Does it support custom network configuration?

**If Jules supports Docker:**
```dockerfile
FROM ubuntu:24.04

# Install Tailscale
RUN curl -fsSL https://tailscale.com/install.sh | sh

# Install bd CLI
RUN curl -fsSL https://raw.githubusercontent.com/steveyegge/beads/main/scripts/install.sh | bash

# Connect on startup
CMD ["sh", "-c", "tailscale up --authkey=${TS_AUTHKEY} && exec /bin/bash"]
```

**If Jules allows apt packages:**
```bash
# Add Tailscale repo
curl -fsSL https://pkgs.tailscale.com/stable/ubuntu/noble.noarmor.gpg \
  -o /usr/share/keyrings/tailscale-archive-keyring.gpg

echo "deb [signed-by=/usr/share/keyrings/tailscale-archive-keyring.gpg] \
  https://pkgs.tailscale.com/stable/ubuntu noble main" \
  | tee /etc/apt/sources.list.d/tailscale.list

apt-get update && apt-get install -y tailscale

# Connect
tailscale up --authkey=$TS_AUTHKEY
```

### 2.4 Other Cloud Environments

**General Pattern:**

1. **Install Tailscale** (platform-specific method)
2. **Connect with auth key:** `tailscale up --authkey=tskey-auth-XXX`
3. **Install bd CLI**
4. **Connect to your server:** `bd --server YOUR_SERVER_IP:50051`

**Platform-specific guides:**

- **GitLab CI:** https://tailscale.com/kb/1207/gitlab-ci
- **CircleCI:** https://tailscale.com/kb/1208/circleci
- **Docker:** https://tailscale.com/kb/1282/docker
- **Kubernetes:** https://tailscale.com/kb/1185/kubernetes

## Part 3: Security Best Practices

### 3.1 Auth Key Management

```bash
# Rotate keys every 90 days
# Store in secure secret management:
# - GitHub Secrets
# - AWS Secrets Manager
# - HashiCorp Vault

# Use different keys for different purposes
# - Production: Long-lived, restricted ACLs
# - Development: Short-lived, broader access
# - CI/CD: Ephemeral, specific services only
```

### 3.2 ACL Best Practices

```json
{
  // Principle of least privilege
  "acls": [
    // Only allow access to specific ports
    {
      "action": "accept",
      "src": ["tag:github-ci"],
      "dst": ["tag:server:50051"]  // Only beads RPC
    },

    // Separate prod and dev
    {
      "action": "accept",
      "src": ["tag:dev-env"],
      "dst": ["tag:dev-server:*"]
    },

    // Deny by default
    {
      "action": "deny",
      "src": ["*"],
      "dst": ["*"]
    }
  ]
}
```

### 3.3 Monitoring

**Enable Tailscale logs:**
- https://login.tailscale.com/admin/logs

**Monitor connections:**
```bash
# On your server
tailscale status

# Watch for ephemeral nodes
watch -n 5 'tailscale status | grep tag:cloud-env'

# Check bd daemon connections
bd daemon --metrics
```

### 3.4 Ephemeral Nodes

Always use `--ephemeral` for cloud environments:
- Nodes auto-remove when disconnected
- Prevents Tailnet clutter
- Better security (no lingering access)

## Part 4: Client Configuration

### 4.1 Environment Variables

**In all cloud environments, set:**

```bash
# Point to your Tailscale server
export BD_SERVER="100.x.x.x:50051"  # Your server's Tailscale IP
# or
export BEADS_DB="100.x.x.x:50051"

# Optional
export BD_ACTOR="github-ci"  # Identify the environment
export BD_DEBUG=1             # Enable debug logging
```

### 4.2 Connection Testing

```bash
# After Tailscale connects
tailscale ip -4  # Get this environment's IP
tailscale ping your-server  # Test connectivity

# Test beads connection
bd --server YOUR_SERVER:50051 list
bd --server YOUR_SERVER:50051 ready
```

## Part 5: Troubleshooting

### Common Issues

**"Connection refused" when connecting to bd server:**
```bash
# Check server is listening
# On server:
netstat -tulpn | grep 50051
# or
ss -tulpn | grep 50051

# Check firewall (Tailscale usually handles this)
sudo ufw status

# Verify ACLs allow connection
# In Tailscale admin console
```

**"Auth key expired":**
```bash
# Generate new key in Tailscale admin
# Update secrets in GitHub/etc.
```

**"Node not appearing in Tailnet":**
```bash
# Check Tailscale status
tailscale status

# Check if daemon is running
systemctl status tailscaled

# Try restarting
sudo tailscale down
sudo tailscale up --authkey=NEW_KEY
```

## Part 6: Your Action Items

### Immediate (Do Now):

1. ✅ **Create reusable auth key** in Tailscale admin
2. ✅ **Configure ACLs** to allow `tag:cloud-env` → your server
3. ✅ **Tag your server** with `tag:server`
4. ✅ **Start bd daemon** on Tailscale interface
5. ✅ **Test with GitHub Actions** (works now!)

### Short-term (This Week):

6. ⏳ **Submit Claude Code feature request** (if you want it for all users)
7. ⏳ **Set up Jules** (once I know what it is)
8. ⏳ **Document your server's Tailscale IP** for environment configs

### Long-term (As Needed):

9. 📅 **Rotate auth keys** every 90 days
10. 📅 **Review ACLs** quarterly
11. 📅 **Monitor usage** via Tailscale admin logs

## Summary

**What Works Now:**
- ✅ GitHub Actions (official support)
- ✅ GitLab CI (official support)
- ✅ Any environment with Docker support
- ✅ Your local machine

**What's Blocked:**
- ❌ Claude Code sandbox (needs Anthropic to whitelist)

**What You Need to Do:**
1. Set up Tailscale auth keys and ACLs (30 minutes)
2. Configure your central server (15 minutes)
3. Add GitHub secrets and update workflows (15 minutes)
4. Tell me about Jules so I can help configure it

**Questions?**
- What is Jules?
- Do you want me to submit the Claude Code feature request?
- Any other environments you need to support?
