# rust-daq Documentation

Welcome to the documentation for the `rust-daq` project. This documentation covers the V5 "Headless-First" architecture with reactive parameters, scriptable control, and remote GUI support.

## 🔍 Searching Documentation

This documentation is **indexed for semantic search** using CocoIndex. Instead of browsing files, you can search by natural language queries to quickly find relevant information.

**Quick Search:**
```bash
# From repository root
python scripts/search_hybrid.py --query "your question" --mode comprehensive
```

**Example Queries:**
- "How does V5 parameter reactive system work?"
- "What are the async hardware callback patterns?"
- "How to integrate Python client with gRPC?"
- "MaiTai laser communication protocol"

**Search Categories:**
- `architecture` - Design decisions, ADRs, V5 architecture
- `guides` - How-to guides, tutorials, development workflows
- `reference` - Hardware protocols, API references, testing strategies
- `instruments` - Device-specific protocols and findings
- `getting_started` - Setup and onboarding
- `tools` - Development tools and utilities

See [Hybrid Search Setup](./HYBRID_SEARCH_SETUP.md) for detailed usage and [CLAUDE.md](../CLAUDE.md#documentation-search--knowledge-base) for integration with AI assistants.

## Documentation Structure

### 🏗️ [Architecture](../../docs/architecture/)
High-level design documents explaining the core principles of the V5 "Headless-First" architecture.
- [System Architecture](../../docs/architecture/ARCHITECTURE.md): Detailed breakdown of system design.
- [Feature Matrix](../../docs/architecture/FEATURE_MATRIX.md): Guide to cargo features and build profiles.
- [PVCAM Driver Architecture](../../docs/architecture/adr-pvcam-driver-architecture.md): PVCAM integration patterns.

### 📚 [Guides](./guides/)
Practical how-to guides for users and developers.
- **For Users**:
  - [CLI Guide](./guides/cli_guide.md): How to use the command-line interface.
  - [Scripting Guide](./guides/scripting/README.md): How to write Rhai scripts for experiments.
- **For Developers**:
  - [Driver Development](./guides/driver_development.md): Implementing new hardware drivers.
  - [Python Integration](./guides/python_integration.md): Using the Python engine.
  - [Agent Guide](./guides/agent_guide.md): Workflows for AI agents.
  - [Tools](./guides/tools/): Helper scripts and tools.

### 📖 [Reference](./reference/)
Technical reference material.
- **API Documentation**:
  - [gRPC API Reference](./reference/grpc_api.md): Complete gRPC service and message documentation.
  - [Python Client API](./reference/python/rust_daq.html): Auto-generated Python API docs (pdoc).
- **Instruments**:
  - [Instruments](./reference/instruments/): Manuals, findings, and protocol details for specific hardware.
  - [PVCAM Validation Checklist](./reference/instruments/PVCAM_VALIDATION_CHECKLIST.md): Prime BSI camera testing.
  - [PVCAM Hardware Validation](./reference/instruments/PVCAM_HARDWARE_VALIDATION.md): SDK installation and validation.
  - [PVCAM Operator Guide](./reference/instruments/PVCAM_OPERATOR_GUIDE.md): Operating the Prime BSI camera.
- [PVCAM SDK](./reference/pvcam-sdk/): Documentation for the PVCAM camera SDK.
