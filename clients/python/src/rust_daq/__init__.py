"""
rust-daq Python Client Library

A Python client for controlling the rust-daq headless daemon via gRPC.

The library provides three layers:
- Layer 0: Auto-generated protobuf stubs (in .generated submodule)
- Layer 1: AsyncClient - Async-first gRPC wrapper
- Layer 2: High-level synchronous API (coming in bd-daun.2)

Example usage:

    import anyio
    from rust_daq import AsyncClient

    async def main():
        async with AsyncClient("localhost:50051") as client:
            devices = await client.list_devices()
            for device in devices:
                print(f"Found device: {device['id']}")

    anyio.run(main)
"""

from ._version import __version__
from .core import AsyncClient
from .exceptions import (
    DaqError,
    DeviceError,
    CommunicationError,
    TimeoutError,
    ConfigurationError,
)

__all__ = [
    "__version__",
    "AsyncClient",
    "DaqError",
    "DeviceError",
    "CommunicationError",
    "TimeoutError",
    "ConfigurationError",
]
