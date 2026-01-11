# MCP SSH Server for Claude Code Web

This MCP server provides SSH access to the `maitai-eos` machine over Tailscale, exposed via Tailscale Funnel for use with Claude Code Web (claude.ai).

## Architecture

```
Claude.ai Web ──HTTPS──► Tailscale Funnel ──► MCP SSH Server ──Tailnet──► maitai-eos
                         (your-mac.ts.net)   (localhost:3000)              (100.117.5.12)
```

## Prerequisites

1. **Tailscale** installed and logged in (`tailscale up`)
2. **Python 3.10+** installed
3. **SSH key** with access to maitai-eos (typically `~/.ssh/id_ed25519`)
4. **Tailscale Funnel** enabled in your tailnet ACLs

### Enable Funnel in Tailscale ACLs

In the [Tailscale Admin Console](https://login.tailscale.com/admin/acls), add:

```json
{
  "nodeAttrs": [
    {
      "target": ["autogroup:member"],
      "attr": ["funnel"]
    }
  ]
}
```

## Quick Start

```bash
cd tools/mcp-ssh-server

# Full setup (first time)
./setup.sh

# The script will output your Funnel URL, e.g.:
# https://your-mac.tailnet-name.ts.net
```

## Adding to Claude.ai

1. Go to [claude.ai](https://claude.ai)
2. Click your profile → **Settings**
3. Navigate to **Connectors** in the sidebar
4. Click **Add custom connector**
5. Enter your Funnel URL: `https://your-mac.tailnet-name.ts.net/mcp/v1/messages`

## Available Tools

Once connected, Claude will have access to these tools:

| Tool | Description |
|------|-------------|
| `ssh_execute` | Execute shell commands on maitai-eos |
| `ssh_read_file` | Read files from maitai-eos |
| `ssh_write_file` | Write files to maitai-eos |
| `ssh_list_directory` | List directory contents |
| `ssh_connection_status` | Check SSH connection status |

## Usage Examples

In Claude.ai, you can ask:

- "Run `cargo test` on the rust-daq project on maitai-eos"
- "Show me the contents of `/home/maitai/rust-daq/Cargo.toml`"
- "List the files in `/dev/` to see available serial ports"
- "Check the status of the PVCAM camera"

## Management

```bash
# Check status
./setup.sh status

# Restart server
./setup.sh restart

# Stop everything
./setup.sh stop

# View logs
tail -f server.log
```

## Configuration

Environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `SSH_HOST` | `maitai-eos` | Primary hostname (MagicDNS) |
| `SSH_FALLBACK_HOST` | `100.117.5.12` | Fallback IP address |
| `SSH_USER` | `maitai` | SSH username |
| `SSH_KEY_PATH` | `~/.ssh/id_ed25519` | Path to SSH private key |
| `MCP_PORT` | `3000` | Local HTTP server port |

## Security Notes

- The MCP server only accepts connections from Tailscale Funnel (HTTPS)
- SSH authentication uses your existing key pair
- Tailscale provides end-to-end encryption within the tailnet
- Consider limiting which commands can be executed for production use

## Troubleshooting

### Funnel not working

```bash
# Check Funnel status
tailscale serve status

# Verify ACLs allow funnel
tailscale status --json | jq '.Self.Capabilities'
```

### SSH connection fails

```bash
# Test SSH directly
ssh maitai@maitai-eos hostname

# Check Tailscale connectivity
tailscale ping maitai-eos
```

### Server won't start

```bash
# Check logs
cat server.log

# Verify port is free
lsof -i :3000
```

## Files

- `ssh_mcp_http_server.py` - HTTP-based MCP server (for Funnel)
- `ssh_mcp_server.py` - Stdio-based MCP server (for CLI)
- `setup.sh` - Setup and management script
- `requirements.txt` - Python dependencies
