# Claude.ai Cloud Environment + Tailscale Setup

Connect Claude Code on the web to your private infrastructure via Tailscale.

## Architecture

```
Claude.ai Cloud VM ──HTTPS/DERP──► Tailscale Network ──► maitai-eos
   (ephemeral)        (via proxy)     (your tailnet)      (hardware)
```

## How It Works

1. **SessionStart Hook**: When the cloud env starts, `scripts/tailscale-cloud-setup.sh` runs automatically
2. **Tailscale Install**: Script installs Tailscale if not present
3. **Auth & Connect**: Uses `TS_AUTHKEY` env var to join your tailnet as ephemeral node
4. **Secure Access**: Claude can now SSH to maitai-eos over the tailnet

## Prerequisites

1. **Claude Max subscription** (for cloud environments)
2. **Tailscale auth key** (ephemeral, reusable, tagged)
3. **This repo** pushed to GitHub (cloud env clones it)

## Step 1: Configure Tailscale ACLs

Add to your [Tailscale ACL](https://login.tailscale.com/admin/acls):

```json
{
  "tagOwners": {
    "tag:claude-cloud": ["autogroup:admin"]
  },
  "acls": [
    {
      "action": "accept",
      "src": ["tag:claude-cloud"],
      "dst": ["maitai-eos:22"]
    }
  ]
}
```

This restricts Claude cloud instances to **only SSH access to maitai-eos**.

## Step 2: Create Auth Key

Go to [Tailscale Admin → Settings → Keys](https://login.tailscale.com/admin/settings/keys):

| Setting | Value |
|---------|-------|
| Reusable | ✅ Yes |
| Ephemeral | ✅ Yes |
| Tags | `tag:claude-cloud` |
| Expiration | 90 days |

Copy the generated key (starts with `tskey-auth-`).

## Step 3: Create Claude Cloud Environment

In Claude.ai → Settings → Cloud Environments → Add:

**Name:** `tailscale-maitai`

**Network access:** `Trusted`

**Environment variables:**
```env
TS_AUTHKEY=tskey-auth-xxxxxxxxxxxx-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
SSH_HOST=maitai-eos
SSH_USER=maitai
```

## Step 4: Use It

1. Push this repo to GitHub (includes the SessionStart hook)
2. Start a Claude Code web session on the repo
3. The hook runs automatically and connects to Tailscale
4. Claude can now SSH to maitai-eos:
   ```bash
   ssh $SSH_USER@$SSH_HOST
   ```

## Files

- `.claude/settings.json` - SessionStart hook configuration
- `tools/mcp-ssh-server/scripts/tailscale-cloud-setup.sh` - Tailscale setup script

## Potential Issues

### 1. Proxy Blocking
Anthropic's proxy may block Tailscale's coordination server or DERP relays.
- **Workaround:** May need to request `*.tailscale.com` and `derp*.tailscale.com` in trusted sources

### 2. No TUN Device
The VM may not allow creating TUN devices.
- **Workaround:** Script tries userspace networking with `--tun=userspace-networking`

### 3. UDP Blocked
WireGuard uses UDP which proxies typically don't support.
- **Workaround:** Tailscale DERP relays use HTTPS fallback automatically

### 4. sudo Not Available
The script uses sudo for tailscaled. If unavailable:
- May need to run Tailscale in rootless/userspace mode

## Security

- Auth key is **ephemeral** - nodes auto-delete when offline
- Auth key is **tagged** - ACLs restrict what it can access
- Only SSH to maitai-eos is permitted (not full network access)
- Cloud env auto-terminates after session
