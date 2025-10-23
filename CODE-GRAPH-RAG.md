# Code Graph RAG Integration

This document describes how the rust-daq codebase is integrated with a code-graph-rag knowledge graph system for AI-powered code analysis.

## Overview

The rust-daq codebase is continuously analyzed by a code-graph-rag system running on the `ai-agents` server (100.126.161.81). This creates a queryable knowledge graph of the entire codebase structure, relationships, and dependencies.

## System Architecture

### Components

- **Memgraph** (port 7687) - Graph database storing code structure
- **Ollama** (port 11434) - Local LLM for natural language queries
- **Realtime File Monitor** - Automatically updates graph when code changes
- **Query Tools** - Direct Cypher query interface

### Knowledge Graph Statistics

```
Total Nodes:        8,497
Total Relations:   10,989
Modules:              169
Classes/Structs:      209
Functions:            703
Methods:              814
```

## Accessing the Knowledge Graph

### From Any Tailscale Device

The knowledge graph is accessible from any device on the Tailscale network:

```python
import mgclient

# Connect to knowledge graph
conn = mgclient.connect(host='100.126.161.81', port=7687)
cursor = conn.cursor()

# Query
cursor.execute("""
    MATCH (m:Module)-[:DEFINES]->(c:Class)
    WHERE c.name = 'DaqApp'
    RETURN c.name, m.path, c.qualified_name
""")

for name, path, qname in cursor.fetchall():
    print(f"{name} in {path}")

conn.close()
```

## Example Queries

### 1. Find All Structs in a Module

```python
import mgclient
conn = mgclient.connect(host='100.126.161.81', port=7687)
cursor = conn.cursor()

cursor.execute("""
    MATCH (m:Module {path: 'src/app.rs'})-[:DEFINES]->(c:Class)
    RETURN c.name ORDER BY c.name
""")

print("Structs in src/app.rs:")
for (struct_name,) in cursor.fetchall():
    print(f"  - {struct_name}")

conn.close()
```

**Expected Output:**
```
Structs in src/app.rs:
  - DaqApp
  - DaqAppCompat
  - DaqDataSender
  - DaqInstruments
  - DaqStorageFormat
```

### 2. Find All Methods of a Struct

```python
import mgclient
conn = mgclient.connect(host='100.126.161.81', port=7687)
cursor = conn.cursor()

cursor.execute("""
    MATCH (c:Class {name: 'DaqApp'})-[:HAS_METHOD]->(m:Method)
    RETURN m.name, m.qualified_name
    ORDER BY m.name
""")

print("DaqApp methods:")
for method_name, qname in cursor.fetchall():
    print(f"  {method_name}")

conn.close()
```

### 3. Trace Function Call Chains

```python
import mgclient
conn = mgclient.connect(host='100.126.161.81', port=7687)
cursor = conn.cursor()

cursor.execute("""
    MATCH (f1:Function)-[:CALLS]->(f2:Function {name: 'initialize'})
    MATCH (m:Module)-[:DEFINES]->(f1)
    RETURN DISTINCT f1.name, m.path
    ORDER BY f1.name
    LIMIT 20
""")

print("Functions that call 'initialize':")
for func_name, file_path in cursor.fetchall():
    print(f"  {func_name} in {file_path}")

conn.close()
```

### 4. Find All Test Functions

```python
import mgclient
conn = mgclient.connect(host='100.126.161.81', port=7687)
cursor = conn.cursor()

cursor.execute("""
    MATCH (m:Module)-[:DEFINES]->(f:Function)
    WHERE f.name STARTS WITH 'test_'
    RETURN f.name, m.path
    ORDER BY f.name
    LIMIT 20
""")

print("Test functions:")
for func_name, file_path in cursor.fetchall():
    print(f"  {func_name} ({file_path})")

conn.close()
```

### 5. Find Dependencies for a Struct

```python
import mgclient
conn = mgclient.connect(host='100.126.161.81', port=7687)
cursor = conn.cursor()

cursor.execute("""
    MATCH (c:Class {name: 'DaqManagerActor'})
    MATCH (c)-[:USES]->(dep)
    RETURN DISTINCT labels(dep)[0] as type, dep.name as name
    ORDER BY type, name
""")

print("DaqManagerActor dependencies:")
for dep_type, dep_name in cursor.fetchall():
    print(f"  {dep_type}: {dep_name}")

conn.close()
```

### 6. Search for Modules by Name

```python
import mgclient
conn = mgclient.connect(host='100.126.161.81', port=7687)
cursor = conn.cursor()

cursor.execute("""
    MATCH (m:Module)
    WHERE m.path CONTAINS 'instrument'
    RETURN m.path
    ORDER BY m.path
""")

print("Instrument-related modules:")
for (path,) in cursor.fetchall():
    print(f"  {path}")

conn.close()
```

## Command-Line Query Tool

For quick queries without writing Python scripts, use the query tool on ai-agents:

```bash
ssh root@100.126.161.81

# Show statistics
cd /root/code-graph-rag
.venv/bin/python3 query-graph.py stats

# List all structs
.venv/bin/python3 query-graph.py structs

# Find specific struct
.venv/bin/python3 query-graph.py struct DaqApp

# Find functions
.venv/bin/python3 query-graph.py function initialize
```

## Realtime Updates

The knowledge graph automatically updates when you modify code:

1. Edit any `.rs` file in the rust-daq repository
2. Save the file
3. The realtime monitor detects the change
4. Graph is updated within seconds
5. Query the graph to see the changes

**Monitor logs:**
```bash
ssh root@100.126.161.81
tail -f /tmp/realtime-updater.log
```

## Use Cases for AI Agents

### Code Understanding
- "What structs are defined in the measurement module?"
- "Show me all functions that call `broadcast`"
- "What are the dependencies of the DaqApp struct?"

### Impact Analysis
- "What functions will be affected if I change the DaqError enum?"
- "Which modules use the Newport1830C instrument?"
- "Show me the call chain from main() to the data storage layer"

### Test Discovery
- "Find all tests related to the power meter module"
- "Show me test coverage for the session management code"
- "Which tests call the `save_session` function?"

### Architecture Analysis
- "What are the core structs in the application layer?"
- "Show me the relationship between instruments and modules"
- "Map out the data flow from instruments to storage"

## Graph Schema

### Node Types

- **Module** - Rust source files and modules
  - Properties: `path`, `name`, `qualified_name`
  
- **Class** - Structs, enums, traits, types
  - Properties: `name`, `qualified_name`, `docstring`
  
- **Function** - Top-level functions
  - Properties: `name`, `qualified_name`, `docstring`
  
- **Method** - Struct/trait methods
  - Properties: `name`, `qualified_name`, `docstring`

### Relationship Types

- **DEFINES** - Module defines a Class/Function
- **HAS_METHOD** - Class has a Method
- **CALLS** - Function/Method calls another Function/Method
- **USES** - References/imports between code elements
- **IMPLEMENTS** - Trait implementation relationships

## Advanced Cypher Queries

### Find Circular Dependencies
```cypher
MATCH path = (m1:Module)-[:USES*2..5]->(m1)
RETURN path
LIMIT 10
```

### Find Most Connected Structs
```cypher
MATCH (c:Class)-[r]-()
RETURN c.name, count(r) as connections
ORDER BY connections DESC
LIMIT 20
```

### Find Orphaned Functions
```cypher
MATCH (f:Function)
WHERE NOT (f)<-[:CALLS]-()
AND f.name <> 'main'
RETURN f.name, f.qualified_name
```

### Find Complex Functions (many calls)
```cypher
MATCH (f:Function)-[:CALLS]->()
WITH f, count(*) as call_count
WHERE call_count > 10
RETURN f.name, f.qualified_name, call_count
ORDER BY call_count DESC
```

## Installation (For Reference)

The code-graph-rag system is already installed and running on ai-agents. If you need to set it up elsewhere:

```bash
# Clone repository
git clone https://github.com/psobolik/code-graph-rag.git
cd code-graph-rag

# Install dependencies
pip install -r requirements.txt

# Start services (Memgraph, Ollama)
# See: https://github.com/psobolik/code-graph-rag

# Parse repository
python3 -m codebase_rag.main start --repo-path /path/to/rust-daq

# Start realtime monitor
python3 realtime_updater.py /path/to/rust-daq
```

## Troubleshooting

### Can't Connect to Graph
```bash
# Test connection
nc -zv 100.126.161.81 7687

# Verify Tailscale connectivity
tailscale status | grep ai-agents
```

### Graph Data Seems Stale
```bash
# Check if realtime monitor is running
ssh root@100.126.161.81 "ps aux | grep realtime_updater"

# Check logs
ssh root@100.126.161.81 "tail -100 /tmp/realtime-updater.log"
```

### Query Performance Issues
```cypher
-- Add indexes for commonly queried properties
CREATE INDEX ON :Module(path);
CREATE INDEX ON :Class(name);
CREATE INDEX ON :Function(name);
```

## Resources

- **Code-Graph-RAG GitHub**: https://github.com/psobolik/code-graph-rag
- **Memgraph Documentation**: https://memgraph.com/docs
- **Cypher Query Language**: https://memgraph.com/docs/cypher-manual
- **Server**: ai-agents (100.126.161.81)
- **Server Logs**: `/tmp/realtime-updater.log`

## Support

For issues with the knowledge graph integration:
1. Check service status on ai-agents
2. Review realtime updater logs
3. Test connectivity from your device
4. Verify Tailscale network access

---

**Last Updated**: 2025-10-23  
**Graph Version**: 8,497 nodes, 10,989 relationships  
**Repository**: rust-daq (84 Rust source files)
