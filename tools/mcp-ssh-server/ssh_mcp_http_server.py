#!/usr/bin/env python3
"""
HTTP-based MCP SSH Server for Claude Code Web access to maitai-eos over Tailscale Funnel.

This server exposes SSH functionality via MCP over HTTP, suitable for
exposure through Tailscale Funnel for Claude.ai web access.
"""

import asyncio
import os
import json
import logging
import uuid
from pathlib import Path
from typing import Any
from contextlib import asynccontextmanager

import asyncssh
from fastapi import FastAPI, Request, Response
from fastapi.responses import StreamingResponse
from sse_starlette.sse import EventSourceResponse
import uvicorn

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Server configuration
DEFAULT_HOST = os.environ.get("SSH_HOST", "maitai-eos")
FALLBACK_HOST = os.environ.get("SSH_FALLBACK_HOST", "100.117.5.12")
DEFAULT_USER = os.environ.get("SSH_USER", "maitai")
SSH_KEY_PATH = os.environ.get("SSH_KEY_PATH", str(Path.home() / ".ssh" / "id_ed25519"))
SERVER_PORT = int(os.environ.get("MCP_PORT", "3000"))

# Connection cache
_connection_cache: dict[str, asyncssh.SSHClientConnection] = {}


async def get_connection(
    host: str = DEFAULT_HOST,
    user: str = DEFAULT_USER,
    key_path: str = SSH_KEY_PATH,
) -> asyncssh.SSHClientConnection:
    """Get or create an SSH connection."""
    cache_key = f"{user}@{host}"

    if cache_key in _connection_cache:
        conn = _connection_cache[cache_key]
        try:
            if conn._transport is not None and not conn._transport.is_closing():
                return conn
        except Exception:
            pass
        del _connection_cache[cache_key]

    hosts_to_try = [host]
    if host == DEFAULT_HOST and FALLBACK_HOST:
        hosts_to_try.append(FALLBACK_HOST)

    last_error = None
    for try_host in hosts_to_try:
        try:
            logger.info(f"Connecting to {user}@{try_host}")
            conn = await asyncssh.connect(
                try_host,
                username=user,
                client_keys=[key_path] if Path(key_path).exists() else None,
                known_hosts=None,
                connect_timeout=10,
            )
            _connection_cache[cache_key] = conn
            logger.info(f"Connected successfully to {try_host}")
            return conn
        except Exception as e:
            logger.warning(f"Failed to connect to {try_host}: {e}")
            last_error = e

    raise ConnectionError(f"Failed to connect to any host: {last_error}")


# Tool definitions
TOOLS = [
    {
        "name": "ssh_execute",
        "description": "Execute a command on the remote maitai-eos machine via SSH over Tailscale",
        "inputSchema": {
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute on the remote machine",
                },
                "working_directory": {
                    "type": "string",
                    "description": "Optional working directory for the command",
                },
                "timeout": {
                    "type": "integer",
                    "description": "Command timeout in seconds (default: 60)",
                    "default": 60,
                },
            },
            "required": ["command"],
        },
    },
    {
        "name": "ssh_read_file",
        "description": "Read a file from the remote maitai-eos machine",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute path to the file to read",
                },
                "max_lines": {
                    "type": "integer",
                    "description": "Maximum number of lines to read (default: 1000)",
                    "default": 1000,
                },
            },
            "required": ["path"],
        },
    },
    {
        "name": "ssh_write_file",
        "description": "Write content to a file on the remote maitai-eos machine",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute path to the file to write",
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file",
                },
            },
            "required": ["path", "content"],
        },
    },
    {
        "name": "ssh_list_directory",
        "description": "List contents of a directory on the remote maitai-eos machine",
        "inputSchema": {
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The absolute path to the directory",
                },
                "show_hidden": {
                    "type": "boolean",
                    "description": "Whether to show hidden files (default: true)",
                    "default": True,
                },
            },
            "required": ["path"],
        },
    },
    {
        "name": "ssh_connection_status",
        "description": "Check the SSH connection status to maitai-eos",
        "inputSchema": {
            "type": "object",
            "properties": {},
        },
    },
]


async def handle_ssh_execute(arguments: dict[str, Any]) -> str:
    """Execute a command on the remote machine."""
    command = arguments["command"]
    working_dir = arguments.get("working_directory")
    timeout = arguments.get("timeout", 60)

    if working_dir:
        command = f"cd {working_dir} && {command}"

    conn = await get_connection()
    result = await asyncio.wait_for(
        conn.run(command, check=False),
        timeout=timeout,
    )

    output = []
    if result.stdout:
        output.append(f"STDOUT:\n{result.stdout}")
    if result.stderr:
        output.append(f"STDERR:\n{result.stderr}")
    output.append(f"Exit code: {result.exit_status}")

    return "\n".join(output)


async def handle_ssh_read_file(arguments: dict[str, Any]) -> str:
    """Read a file from the remote machine."""
    path = arguments["path"]
    max_lines = arguments.get("max_lines", 1000)

    conn = await get_connection()
    result = await conn.run(f"head -n {max_lines} '{path}'", check=False)

    if result.exit_status != 0:
        return f"Error reading file: {result.stderr}"

    return result.stdout


async def handle_ssh_write_file(arguments: dict[str, Any]) -> str:
    """Write content to a file on the remote machine."""
    path = arguments["path"]
    content = arguments["content"]

    conn = await get_connection()

    async with conn.start_sftp_client() as sftp:
        async with sftp.open(path, "w") as f:
            await f.write(content)

    return f"Successfully wrote to {path}"


async def handle_ssh_list_directory(arguments: dict[str, Any]) -> str:
    """List directory contents on the remote machine."""
    path = arguments["path"]
    show_hidden = arguments.get("show_hidden", True)

    ls_flags = "-la" if show_hidden else "-l"

    conn = await get_connection()
    result = await conn.run(f"ls {ls_flags} '{path}'", check=False)

    if result.exit_status != 0:
        return f"Error listing directory: {result.stderr}"

    return result.stdout


async def handle_ssh_connection_status(arguments: dict[str, Any]) -> str:
    """Check SSH connection status."""
    try:
        conn = await get_connection()
        result = await conn.run("hostname && uname -a", check=False)

        return (
            f"Connected to maitai-eos\n"
            f"Host: {DEFAULT_HOST} (fallback: {FALLBACK_HOST})\n"
            f"User: {DEFAULT_USER}\n"
            f"System info:\n{result.stdout}"
        )
    except Exception as e:
        return (
            f"Connection failed: {str(e)}\n"
            f"Host: {DEFAULT_HOST} (fallback: {FALLBACK_HOST})\n"
            f"User: {DEFAULT_USER}"
        )


TOOL_HANDLERS = {
    "ssh_execute": handle_ssh_execute,
    "ssh_read_file": handle_ssh_read_file,
    "ssh_write_file": handle_ssh_write_file,
    "ssh_list_directory": handle_ssh_list_directory,
    "ssh_connection_status": handle_ssh_connection_status,
}


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Application lifespan handler."""
    logger.info("Starting SSH MCP HTTP Server for maitai-eos")
    logger.info(f"Target host: {DEFAULT_HOST} (fallback: {FALLBACK_HOST})")
    logger.info(f"SSH user: {DEFAULT_USER}")
    logger.info(f"Listening on port {SERVER_PORT}")
    yield
    # Cleanup connections
    for conn in _connection_cache.values():
        try:
            conn.close()
        except Exception:
            pass
    logger.info("Server shutdown complete")


app = FastAPI(lifespan=lifespan, title="SSH MCP Server")


@app.get("/")
async def root():
    """Health check endpoint."""
    return {"status": "ok", "service": "ssh-mcp-server", "target": DEFAULT_HOST}


@app.post("/mcp/v1/messages")
async def handle_mcp_message(request: Request):
    """Handle MCP JSON-RPC messages (Streamable HTTP transport)."""
    body = await request.json()
    logger.debug(f"Received MCP message: {json.dumps(body)[:500]}")

    method = body.get("method")
    params = body.get("params", {})
    request_id = body.get("id")

    response = {"jsonrpc": "2.0", "id": request_id}

    try:
        if method == "initialize":
            response["result"] = {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {"listChanged": False},
                },
                "serverInfo": {
                    "name": "ssh-mcp-server",
                    "version": "1.0.0",
                },
            }

        elif method == "tools/list":
            response["result"] = {"tools": TOOLS}

        elif method == "tools/call":
            tool_name = params.get("name")
            tool_args = params.get("arguments", {})

            if tool_name not in TOOL_HANDLERS:
                response["error"] = {
                    "code": -32601,
                    "message": f"Unknown tool: {tool_name}",
                }
            else:
                try:
                    result = await TOOL_HANDLERS[tool_name](tool_args)
                    response["result"] = {
                        "content": [{"type": "text", "text": result}],
                        "isError": False,
                    }
                except Exception as e:
                    logger.exception(f"Error executing tool {tool_name}")
                    response["result"] = {
                        "content": [{"type": "text", "text": f"Error: {str(e)}"}],
                        "isError": True,
                    }

        elif method == "notifications/initialized":
            # Notification, no response needed
            return Response(status_code=204)

        else:
            response["error"] = {
                "code": -32601,
                "message": f"Method not found: {method}",
            }

    except Exception as e:
        logger.exception("Error handling MCP message")
        response["error"] = {"code": -32603, "message": str(e)}

    return response


@app.get("/mcp/v1/sse")
async def handle_mcp_sse(request: Request):
    """Handle MCP SSE connections (Server-Sent Events transport)."""

    async def event_generator():
        # Send initial connection event
        yield {
            "event": "open",
            "data": json.dumps({"sessionId": str(uuid.uuid4())}),
        }

        # Keep connection alive
        while True:
            await asyncio.sleep(30)
            yield {"event": "ping", "data": ""}

    return EventSourceResponse(event_generator())


if __name__ == "__main__":
    uvicorn.run(
        "ssh_mcp_http_server:app",
        host="0.0.0.0",
        port=SERVER_PORT,
        reload=False,
        log_level="info",
    )
