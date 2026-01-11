#!/usr/bin/env python3
"""
MCP SSH Server for Claude Code Web access to maitai-eos over Tailscale.

This server exposes SSH functionality via the Model Context Protocol (MCP),
allowing Claude to execute commands on remote machines through Tailscale.
"""

import asyncio
import os
import logging
from pathlib import Path
from typing import Any

import asyncssh
from mcp.server import Server
from mcp.server.stdio import stdio_server
from mcp.types import Tool, TextContent

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Server configuration
DEFAULT_HOST = os.environ.get("SSH_HOST", "maitai-eos")
FALLBACK_HOST = os.environ.get("SSH_FALLBACK_HOST", "100.117.5.12")
DEFAULT_USER = os.environ.get("SSH_USER", "maitai")
SSH_KEY_PATH = os.environ.get("SSH_KEY_PATH", str(Path.home() / ".ssh" / "id_ed25519"))

# Create the MCP server
app = Server("ssh-remote")

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
        # Check if connection is still alive
        try:
            # Simple check - try to get transport info
            if conn._transport is not None and not conn._transport.is_closing():
                return conn
        except Exception:
            pass
        # Connection is dead, remove from cache
        del _connection_cache[cache_key]

    # Try primary host first, then fallback
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
                known_hosts=None,  # Accept all host keys (Tailscale handles trust)
                connect_timeout=10,
            )
            _connection_cache[cache_key] = conn
            logger.info(f"Connected successfully to {try_host}")
            return conn
        except Exception as e:
            logger.warning(f"Failed to connect to {try_host}: {e}")
            last_error = e

    raise ConnectionError(f"Failed to connect to any host: {last_error}")


@app.list_tools()
async def list_tools() -> list[Tool]:
    """List available SSH tools."""
    return [
        Tool(
            name="ssh_execute",
            description="Execute a command on the remote maitai-eos machine via SSH over Tailscale",
            inputSchema={
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute on the remote machine",
                    },
                    "working_directory": {
                        "type": "string",
                        "description": "Optional working directory for the command",
                        "default": None,
                    },
                    "timeout": {
                        "type": "integer",
                        "description": "Command timeout in seconds (default: 60)",
                        "default": 60,
                    },
                },
                "required": ["command"],
            },
        ),
        Tool(
            name="ssh_read_file",
            description="Read a file from the remote maitai-eos machine",
            inputSchema={
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
        ),
        Tool(
            name="ssh_write_file",
            description="Write content to a file on the remote maitai-eos machine",
            inputSchema={
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
        ),
        Tool(
            name="ssh_list_directory",
            description="List contents of a directory on the remote maitai-eos machine",
            inputSchema={
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
        ),
        Tool(
            name="ssh_connection_status",
            description="Check the SSH connection status to maitai-eos",
            inputSchema={
                "type": "object",
                "properties": {},
            },
        ),
    ]


@app.call_tool()
async def call_tool(name: str, arguments: dict[str, Any]) -> list[TextContent]:
    """Handle tool calls."""
    try:
        if name == "ssh_execute":
            return await handle_ssh_execute(arguments)
        elif name == "ssh_read_file":
            return await handle_ssh_read_file(arguments)
        elif name == "ssh_write_file":
            return await handle_ssh_write_file(arguments)
        elif name == "ssh_list_directory":
            return await handle_ssh_list_directory(arguments)
        elif name == "ssh_connection_status":
            return await handle_ssh_connection_status(arguments)
        else:
            return [TextContent(type="text", text=f"Unknown tool: {name}")]
    except Exception as e:
        logger.exception(f"Error executing tool {name}")
        return [TextContent(type="text", text=f"Error: {str(e)}")]


async def handle_ssh_execute(arguments: dict[str, Any]) -> list[TextContent]:
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

    return [TextContent(type="text", text="\n".join(output))]


async def handle_ssh_read_file(arguments: dict[str, Any]) -> list[TextContent]:
    """Read a file from the remote machine."""
    path = arguments["path"]
    max_lines = arguments.get("max_lines", 1000)

    conn = await get_connection()
    result = await conn.run(f"head -n {max_lines} '{path}'", check=False)

    if result.exit_status != 0:
        return [TextContent(type="text", text=f"Error reading file: {result.stderr}")]

    return [TextContent(type="text", text=result.stdout)]


async def handle_ssh_write_file(arguments: dict[str, Any]) -> list[TextContent]:
    """Write content to a file on the remote machine."""
    path = arguments["path"]
    content = arguments["content"]

    conn = await get_connection()

    # Use SFTP for reliable file writing
    async with conn.start_sftp_client() as sftp:
        async with sftp.open(path, "w") as f:
            await f.write(content)

    return [TextContent(type="text", text=f"Successfully wrote to {path}")]


async def handle_ssh_list_directory(arguments: dict[str, Any]) -> list[TextContent]:
    """List directory contents on the remote machine."""
    path = arguments["path"]
    show_hidden = arguments.get("show_hidden", True)

    ls_flags = "-la" if show_hidden else "-l"

    conn = await get_connection()
    result = await conn.run(f"ls {ls_flags} '{path}'", check=False)

    if result.exit_status != 0:
        return [TextContent(type="text", text=f"Error listing directory: {result.stderr}")]

    return [TextContent(type="text", text=result.stdout)]


async def handle_ssh_connection_status(arguments: dict[str, Any]) -> list[TextContent]:
    """Check SSH connection status."""
    try:
        conn = await get_connection()
        result = await conn.run("hostname && uname -a", check=False)

        return [TextContent(
            type="text",
            text=f"Connected to maitai-eos\n"
                 f"Host: {DEFAULT_HOST} (fallback: {FALLBACK_HOST})\n"
                 f"User: {DEFAULT_USER}\n"
                 f"System info:\n{result.stdout}"
        )]
    except Exception as e:
        return [TextContent(
            type="text",
            text=f"Connection failed: {str(e)}\n"
                 f"Host: {DEFAULT_HOST} (fallback: {FALLBACK_HOST})\n"
                 f"User: {DEFAULT_USER}"
        )]


async def main():
    """Run the MCP server."""
    logger.info("Starting SSH MCP Server for maitai-eos")
    logger.info(f"Target host: {DEFAULT_HOST} (fallback: {FALLBACK_HOST})")
    logger.info(f"SSH user: {DEFAULT_USER}")
    logger.info(f"SSH key: {SSH_KEY_PATH}")

    async with stdio_server() as (read_stream, write_stream):
        await app.run(read_stream, write_stream, app.create_initialization_options())


if __name__ == "__main__":
    asyncio.run(main())
