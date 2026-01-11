#!/bin/bash
#
# Tailscale Setup for Claude.ai Cloud Environments
#
# This script is called by SessionStart hook to connect the cloud VM
# to your Tailscale network for secure access to private infrastructure.
#
# Required environment variables (set in Claude.ai cloud environment):
#   TS_AUTHKEY - Tailscale auth key (ephemeral, tagged)
#
# Optional environment variables:
#   TS_HOSTNAME - Custom hostname for this node (default: claude-cloud-$RANDOM)
#   SSH_HOST - Target SSH host on tailnet (for verification)
#

set -e

LOG_PREFIX="[tailscale-setup]"

log_info() { echo "$LOG_PREFIX INFO: $1"; }
log_warn() { echo "$LOG_PREFIX WARN: $1"; }
log_error() { echo "$LOG_PREFIX ERROR: $1" >&2; }

# Check if we're in a cloud environment
if [[ -z "$CLAUDE_ENV_FILE" ]]; then
    log_info "Not in cloud environment (CLAUDE_ENV_FILE not set), skipping Tailscale setup"
    exit 0
fi

# Check for auth key
if [[ -z "$TS_AUTHKEY" ]]; then
    log_info "TS_AUTHKEY not set, skipping Tailscale setup"
    exit 0
fi

log_info "Starting Tailscale setup for Claude cloud environment"

# Check if Tailscale is already installed
if command -v tailscale &> /dev/null; then
    log_info "Tailscale already installed"
else
    log_info "Installing Tailscale..."
    curl -fsSL https://tailscale.com/install.sh | sh
fi

# Generate hostname if not provided
TS_HOSTNAME="${TS_HOSTNAME:-claude-cloud-$(date +%s | tail -c 6)}"

# Start Tailscale daemon if not running
if ! pgrep -x tailscaled > /dev/null; then
    log_info "Starting tailscaled..."
    # Try to start in userspace mode (doesn't require TUN device)
    sudo tailscaled --state=/var/lib/tailscale/tailscaled.state --tun=userspace-networking &
    sleep 2
fi

# Connect to Tailscale network
log_info "Connecting to Tailscale network as '$TS_HOSTNAME'..."
sudo tailscale up \
    --authkey="$TS_AUTHKEY" \
    --hostname="$TS_HOSTNAME" \
    --accept-routes \
    --ssh \
    2>&1 || {
        log_warn "Standard tailscale up failed, trying with --netfilter-mode=off..."
        sudo tailscale up \
            --authkey="$TS_AUTHKEY" \
            --hostname="$TS_HOSTNAME" \
            --accept-routes \
            --netfilter-mode=off \
            2>&1
    }

# Wait for connection
log_info "Waiting for Tailscale connection..."
for i in {1..30}; do
    if tailscale status --json 2>/dev/null | grep -q '"BackendState":"Running"'; then
        log_info "Tailscale connected!"
        break
    fi
    sleep 1
done

# Show status
tailscale status

# Verify SSH connectivity if SSH_HOST is set
if [[ -n "$SSH_HOST" ]]; then
    log_info "Testing SSH connectivity to $SSH_HOST..."
    if tailscale ping "$SSH_HOST" --timeout=5s &>/dev/null; then
        log_info "SSH host $SSH_HOST is reachable on tailnet"

        # Export SSH variables to CLAUDE_ENV_FILE
        echo "export TAILSCALE_CONNECTED=1" >> "$CLAUDE_ENV_FILE"
        echo "export SSH_HOST=$SSH_HOST" >> "$CLAUDE_ENV_FILE"
        [[ -n "$SSH_USER" ]] && echo "export SSH_USER=$SSH_USER" >> "$CLAUDE_ENV_FILE"
    else
        log_warn "SSH host $SSH_HOST not reachable (may take a moment for DNS)"
    fi
fi

log_info "Tailscale setup complete"
