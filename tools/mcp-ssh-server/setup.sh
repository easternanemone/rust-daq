#!/bin/bash
#
# Setup script for MCP SSH Server with Tailscale Funnel
#
# This script:
# 1. Creates a Python virtual environment
# 2. Installs dependencies
# 3. Configures and starts Tailscale Funnel
# 4. Starts the MCP HTTP server
#
# Usage:
#   ./setup.sh          # Full setup and start
#   ./setup.sh start    # Just start the server (after initial setup)
#   ./setup.sh stop     # Stop the server and funnel
#   ./setup.sh status   # Check status
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VENV_DIR="$SCRIPT_DIR/.venv"
PID_FILE="$SCRIPT_DIR/.server.pid"
LOG_FILE="$SCRIPT_DIR/server.log"
MCP_PORT="${MCP_PORT:-3000}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check Python
    if ! command -v python3 &> /dev/null; then
        log_error "Python 3 is required but not installed"
        exit 1
    fi

    # Check Tailscale
    if ! command -v tailscale &> /dev/null; then
        log_error "Tailscale is required but not installed"
        echo "Install from: https://tailscale.com/download"
        exit 1
    fi

    # Check Tailscale is running
    if ! tailscale status &> /dev/null; then
        log_error "Tailscale is not running or not logged in"
        echo "Run: tailscale up"
        exit 1
    fi

    # Check SSH key exists
    SSH_KEY="${SSH_KEY_PATH:-$HOME/.ssh/id_ed25519}"
    if [[ ! -f "$SSH_KEY" ]]; then
        log_warn "SSH key not found at $SSH_KEY"
        log_warn "Make sure you have SSH key access to maitai-eos"
    fi

    log_info "Prerequisites check passed"
}

setup_venv() {
    log_info "Setting up Python virtual environment..."

    if [[ ! -d "$VENV_DIR" ]]; then
        python3 -m venv "$VENV_DIR"
    fi

    source "$VENV_DIR/bin/activate"
    pip install --upgrade pip -q
    pip install -r "$SCRIPT_DIR/requirements.txt" -q

    log_info "Virtual environment ready"
}

get_tailscale_hostname() {
    tailscale status --json | python3 -c "import sys, json; d=json.load(sys.stdin); print(d['Self']['DNSName'].rstrip('.'))"
}

start_server() {
    log_info "Starting MCP SSH HTTP server..."

    if [[ -f "$PID_FILE" ]]; then
        if kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
            log_warn "Server already running (PID: $(cat "$PID_FILE"))"
            return 0
        fi
        rm "$PID_FILE"
    fi

    source "$VENV_DIR/bin/activate"

    # Start server in background
    nohup python3 "$SCRIPT_DIR/ssh_mcp_http_server.py" > "$LOG_FILE" 2>&1 &
    echo $! > "$PID_FILE"

    sleep 2

    if kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        log_info "Server started (PID: $(cat "$PID_FILE"))"
    else
        log_error "Server failed to start. Check $LOG_FILE"
        exit 1
    fi
}

start_funnel() {
    log_info "Starting Tailscale Funnel on port $MCP_PORT..."

    # Check if funnel is already running
    if tailscale serve status 2>/dev/null | grep -q "$MCP_PORT"; then
        log_warn "Funnel already configured for port $MCP_PORT"
    else
        # Start funnel in background
        tailscale funnel --bg "$MCP_PORT"
    fi

    # Get the public URL
    HOSTNAME=$(get_tailscale_hostname)
    PUBLIC_URL="https://${HOSTNAME}"

    log_info "Tailscale Funnel active"
    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}MCP SSH Server is now accessible at:${NC}"
    echo -e "${YELLOW}  $PUBLIC_URL${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo "Add this URL to Claude.ai:"
    echo "  1. Go to claude.ai → Settings → Connectors"
    echo "  2. Click 'Add custom connector'"
    echo "  3. Enter: ${PUBLIC_URL}/mcp/v1/messages"
    echo ""
}

stop_server() {
    log_info "Stopping server..."

    if [[ -f "$PID_FILE" ]]; then
        if kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
            kill "$(cat "$PID_FILE")"
            log_info "Server stopped"
        fi
        rm -f "$PID_FILE"
    else
        log_warn "No server PID file found"
    fi
}

stop_funnel() {
    log_info "Stopping Tailscale Funnel..."
    tailscale funnel off "$MCP_PORT" 2>/dev/null || true
    log_info "Funnel stopped"
}

show_status() {
    echo ""
    echo "=== MCP SSH Server Status ==="
    echo ""

    # Server status
    if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        echo -e "Server: ${GREEN}Running${NC} (PID: $(cat "$PID_FILE"))"
    else
        echo -e "Server: ${RED}Stopped${NC}"
    fi

    # Funnel status
    echo ""
    echo "=== Tailscale Funnel Status ==="
    tailscale serve status 2>/dev/null || echo "No funnel configured"

    # Connection test
    if [[ -f "$PID_FILE" ]] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        echo ""
        echo "=== Connection Test ==="
        curl -s "http://localhost:$MCP_PORT/" | python3 -m json.tool 2>/dev/null || echo "Server not responding"
    fi

    echo ""
}

case "${1:-full}" in
    full)
        check_prerequisites
        setup_venv
        start_server
        start_funnel
        ;;
    start)
        source "$VENV_DIR/bin/activate" 2>/dev/null || setup_venv
        start_server
        start_funnel
        ;;
    stop)
        stop_server
        stop_funnel
        ;;
    status)
        show_status
        ;;
    restart)
        stop_server
        stop_funnel
        sleep 1
        start_server
        start_funnel
        ;;
    *)
        echo "Usage: $0 {full|start|stop|status|restart}"
        echo ""
        echo "  full    - Full setup: create venv, install deps, start server & funnel"
        echo "  start   - Start server and funnel (assumes setup done)"
        echo "  stop    - Stop server and funnel"
        echo "  status  - Show current status"
        echo "  restart - Stop and start again"
        exit 1
        ;;
esac
