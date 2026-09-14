# Token Optimization Integration Report

## Installation Summary

### Semble
- **Installation Method:** uvx (on-demand execution via Python package)
- **Command:** `uvx --from "semble[mcp]" semble`
- **MCP Server Name:** `semble`
- **Location:** Configured in `~/.config/devin/mcp_config.json`
- **Tools Provided:** `search`, `find_related`

### Graft
- **Installation Method:** npm global package
- **Command:** `graft mcp`
- **MCP Server Name:** `graft`
- **Location:** Configured in `~/.config/devin/mcp_config.json`
- **Tools Provided:** `find_code`, `trace_calls`, `find_all`, `file_api`, `repo_map`, `check_freshness`
- **Graph Location:** `/home/long/HYDRAGROW/graft/` (gitignored)

## MCP Configuration

Both servers are configured in `~/.config/devin/mcp_config.json`:

```json
{
  "mcpServers": {
    "semble": {
      "command": "uvx",
      "args": [
        "--from",
        "semble[mcp]",
        "semble"
      ]
    },
    "graft": {
      "command": "graft",
      "args": [
        "mcp"
      ]
    }
  },
  "permissions": {
    "allow": [
      "mcp__semble__search",
      "mcp__semble__find_related",
      "mcp__graft__find_code",
      "mcp__graft__trace_calls",
      "mcp__graft__find_all",
      "mcp__graft__file_api",
      "mcp__graft__repo_map",
      "mcp__graft__check_freshness"
    ]
  }
}
```

## Usage Strategy and Tool Selection Logic

### When to Use Semble
- **Specific code queries:** Finding functions, classes, implementations
- **Example:** "Find the authentication middleware"
- **Tool:** `mcp__semble__search` with natural language or code query
- **Follow-up:** `mcp__semble__find_related` to explore similar code

### When to Use Graft
- **Architectural understanding:** Subsystems, dependencies, relationships
- **Example:** "What are the main components and how do they relate?"
- **Tools:** `mcp__graft__repo_map` (overview), `mcp__graft__file_api` (file-specific), `mcp__graft__trace_calls` (call graph analysis)

### When to Read Files Directly
- **Complete file context needed:** When editing entire files
- **Configuration files:** When examining non-indexed files
- **Fallback:** When Semble/Graft results are insufficient

## Example Tool Calls

### Semble Search Example
```
Query: "authentication middleware implementation"
Tool: mcp__semble__search
Parameters: {"query": "authentication middleware", "repo": "/home/long/HYDRAGROW"}
Result: Relevant code snippets from hydragrow-backend/src/auth/
```

### Graft Repo Map Example
```
Query: "show codebase architecture"
Tool: mcp__graft__repo_map
Parameters: {}
Result: Ranked tree showing hydragrow-backend, hydragrow-frontend, hydragrow-shared, etc.
```

### Graft Find Code Example
```
Query: "find User class definition"
Tool: mcp__graft__find_code
Parameters: {"query": "User"}
Result: File locations and line numbers for User class definitions
```

### Graft Trace Calls Example
```
Query: "trace who calls authenticate function"
Tool: mcp__graft__trace_calls
Parameters: {"symbol": "authenticate"}
Result: Call graph showing all callers of authenticate
```

## Token Savings Estimate

Based on Semble benchmarks:
- **Traditional approach:** grep + read entire files = ~10,000 tokens per query
- **Semble approach:** targeted snippets = ~200 tokens per query
- **Estimated savings:** ~98% reduction in token usage for code retrieval

Graft provides additional savings by:
- Eliminating redundant architectural exploration across sessions
- Providing cached structural context (~2K tokens for 100K LOC vs repeated grep)
- Enabling impact analysis without full codebase traversal

## Limitations and Considerations

### Semble Limitations
- Requires initial indexing (though fast: ~250ms for average repo)
- Semantic search may miss exact string matches (use grep as fallback)
- Index is session-based; rebuilds on file changes (automatic)

### Graft Limitations
- Initial graph build requires parsing entire codebase (tree-sitter only, no LLM)
- Graph accuracy depends on tree-sitter language support
- Uses `graft mcp` command (not `graft serve` as originally planned)

### Compatibility Notes
- No conflicts with existing MCPs (none were present)
- No changes to model/provider/API keys
- Uses local CPU execution; no external APIs
- Compatible with existing Devin CLI setup

## Testing Results

All tests passed:
1. ✅ Semble search returned relevant authentication code
2. ✅ Semble find_related found similar implementations
3. ✅ Graft repo_map showed correct subsystem structure
4. ✅ Graft file_api provided file dependencies
5. ✅ Graft find_code located User definitions
6. ✅ Graft trace_calls identified call relationships
7. ✅ Tool selection logic prioritized appropriate tools
8. ✅ No conflicts with existing setup

## Post-Installation Verification

The following queries were tested successfully:
- Search for specific function/class → Semble used correctly
- Trace dependencies/relationships → Graft used correctly
- Locate feature-related code → Semble used correctly
- Understand architecture → Graft used correctly

## Exclusion Patterns

Semble indexing excludes the following patterns (configured in `.sembleignore`):
- Build artifacts: `target/`, `dist/`, `build/`, `*.o`, `*.a`, `*.so`, `*.dylib`, `*.dll`, `*.exe`
- Dependencies: `node_modules/`, `vendor/`, `.git/`
- Generated files: `*.generated.*`, `*.pb.go`, `*.pb.rs`, `*.gen.ts`, `*.gen.js`
- Cache and temporary: `.cache/`, `tmp/`, `temp/`, `*.tmp`, `*.swp`, `*.swo`
- IDE and editor files: `.vscode/`, `.idea/`, `*.iml`
- Test coverage: `coverage/`, `.nyc_output/`
- Worktrees: `.worktrees/`
- Documentation: `docs/`, `*.md` (can be read directly if needed)

## Conclusion

Both Semble and Graft are successfully integrated as MCP servers. The agent now has access to:
- Fast semantic code search (Semble) for specific code queries
- Structural codebase mapping (Graft) for architectural understanding
- Token-optimized retrieval (98% reduction estimated)
- Cached indices that auto-update on code changes

The integration maintains full compatibility with the existing setup and provides clear tool selection logic documented in AGENTS.md.
