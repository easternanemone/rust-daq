#!/usr/bin/env python3
"""
MCP Local Command Server for maitai-eos.

This server runs ON maitai-eos and executes commands locally,
exposed via Tailscale Funnel for Claude.ai web access.
"""

import asyncio
import os
import json
import logging
import uuid
from pathlib import Path
from typing import Any
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request, Response
from sse_starlette.sse import EventSourceResponse
import uvicorn

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

# Server configuration
SERVER_PORT = int(os.environ.get("MCP_PORT", "3000"))
HOSTNAME = os.environ.get("HOSTNAME", "maitai-eos")

# Tool definitions
TOOLS = [
    {
        "name": "execute",
        "description": f"Execute a shell command on {HOSTNAME}",
        "inputSchema": {
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute",
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
        "name": "read_file",
        "description": f"Read a file from {HOSTNAME}",
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
        "name": "write_file",
        "description": f"Write content to a file on {HOSTNAME}",
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
        "name": "list_directory",
        "description": f"List contents of a directory on {HOSTNAME}",
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
        "name": "system_status",
        "description": f"Get system status of {HOSTNAME}",
        "inputSchema": {
            "type": "object",
            "properties": {},
        },
    },
]


async def run_command(command: str, timeout: int = 60, cwd: str = None) -> tuple[str, str, int]:
    """Run a shell command and return stdout, stderr, exit_code."""
    try:
        proc = await asyncio.create_subprocess_shell(
            command,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            cwd=cwd,
        )
        stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=timeout)
        return stdout.decode(), stderr.decode(), proc.returncode
    except asyncio.TimeoutError:
        proc.kill()
        return "", f"Command timed out after {timeout} seconds", -1
    except Exception as e:
        return "", str(e), -1


async def handle_execute(arguments: dict[str, Any]) -> str:
    """Execute a command locally."""
    command = arguments["command"]
    working_dir = arguments.get("working_directory")
    timeout = arguments.get("timeout", 60)

    stdout, stderr, exit_code = await run_command(command, timeout, working_dir)

    output = []
    if stdout:
        output.append(f"STDOUT:\n{stdout}")
    if stderr:
        output.append(f"STDERR:\n{stderr}")
    output.append(f"Exit code: {exit_code}")

    return "\n".join(output)


async def handle_read_file(arguments: dict[str, Any]) -> str:
    """Read a file locally."""
    path = arguments["path"]
    max_lines = arguments.get("max_lines", 1000)

    try:
        with open(path, "r") as f:
            lines = f.readlines()[:max_lines]
            return "".join(lines)
    except Exception as e:
        return f"Error reading file: {e}"


async def handle_write_file(arguments: dict[str, Any]) -> str:
    """Write content to a file locally."""
    path = arguments["path"]
    content = arguments["content"]

    try:
        # Create parent directories if needed
        Path(path).parent.mkdir(parents=True, exist_ok=True)
        with open(path, "w") as f:
            f.write(content)
        return f"Successfully wrote to {path}"
    except Exception as e:
        return f"Error writing file: {e}"


async def handle_list_directory(arguments: dict[str, Any]) -> str:
    """List directory contents locally."""
    path = arguments["path"]
    show_hidden = arguments.get("show_hidden", True)

    ls_flags = "-la" if show_hidden else "-l"
    stdout, stderr, exit_code = await run_command(f"ls {ls_flags} '{path}'")

    if exit_code != 0:
        return f"Error listing directory: {stderr}"
    return stdout


async def handle_system_status(arguments: dict[str, Any]) -> str:
    """Get system status."""
    stdout, _, _ = await run_command("hostname && uname -a && uptime && df -h / && free -h")
    return f"System Status for {HOSTNAME}:\n{stdout}"


TOOL_HANDLERS = {
    "execute": handle_execute,
    "read_file": handle_read_file,
    "write_file": handle_write_file,
    "list_directory": handle_list_directory,
    "system_status": handle_system_status,
}


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Application lifespan handler."""
    logger.info(f"Starting MCP Local Server on {HOSTNAME}")
    logger.info(f"Listening on port {SERVER_PORT}")
    yield
    logger.info("Server shutdown complete")


app = FastAPI(lifespan=lifespan, title="MCP Local Server")


@app.get("/")
async def root():
    """Health check endpoint."""
    return {"status": "ok", "service": "mcp-local-server", "host": HOSTNAME}


@app.post("/mcp/v1/messages")
async def handle_mcp_message(request: Request):
    """Handle MCP JSON-RPC messages."""
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
                    "name": "mcp-local-server",
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
    """Handle MCP SSE connections."""
    async def event_generator():
        yield {"event": "open", "data": json.dumps({"sessionId": str(uuid.uuid4())})}
        while True:
            await asyncio.sleep(30)
            yield {"event": "ping", "data": ""}

    return EventSourceResponse(event_generator())


if __name__ == "__main__":
    uvicorn.run(
        "local_mcp_server:app",
        host="0.0.0.0",
        port=SERVER_PORT,
        reload=False,
        log_level="info",
    )
