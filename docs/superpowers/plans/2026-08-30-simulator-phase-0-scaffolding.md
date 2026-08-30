# Simulator Phase 0 - Scaffolding Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the scaffolding for the `hydragrow-simulator` crate and integrate it into the repository's conventions and CI.

**Architecture:** Create a new standard Rust binary/library crate `hydragrow-simulator` that depends on `hydragrow-controller-core` and `hydragrow-shared`. Update the documentation and CI workflows to enforce quality checks.

**Tech Stack:** Rust, Cargo, GitHub Actions, Markdown.

---

### Task 1: Initialize the simulator crate

**Files:**
- Create: `hydragrow-simulator/Cargo.toml`
- Create: `hydragrow-simulator/src/lib.rs`
- Create: `hydragrow-simulator/src/main.rs`

- [ ] **Step 1: Create the Cargo.toml file**

```toml
[package]
name = "hydragrow-simulator"
version = "0.1.0"
edition = "2024"

[dependencies]
hydragrow-controller-core = { path = "../hydragrow-controller-core" }
hydragrow-shared = { path = "../hydragrow-shared" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
clap = { version = "4.4", features = ["derive"] }
tracing = "0.1"
```

- [ ] **Step 2: Create the minimal src/lib.rs**

```rust
// Simulator library root
```

- [ ] **Step 3: Create the minimal src/main.rs**

```rust
fn main() {
    println!("hydragrow-simulator boot");
}
```

- [ ] **Step 4: Verify the crate builds**

Run: `cd hydragrow-simulator && cargo build`
Expected: Successful build.

- [ ] **Step 5: Commit**

```bash
git add hydragrow-simulator/
git commit -m "feat(simulator): initialize crate scaffolding"
```

### Task 2: Add module rules for the simulator

**Files:**
- Create: `docs/superpowers/specs/module-rules/simulator.md`

- [ ] **Step 1: Create the module rules file**

```markdown
# Simulator Rules

1. **Host-native only:** This crate runs on the host (Linux/macOS/Windows). Do NOT add dependencies on `esp-idf-sys` or any ESP32-specific hardware crates.
2. **Dependencies:** Can only depend on `hydragrow-controller-core` and `hydragrow-shared`. Do NOT depend on `hydragrow-backend` or `ESP32-C3-CONTROLLER-NODE`.
3. **Testing:** All behavior (Plant model, Fault injection, Scenarios) MUST be fully tested via standard `cargo test` unit tests or snapshot tests (`insta`).
```

- [ ] **Step 2: Commit**

```bash
git add docs/superpowers/specs/module-rules/simulator.md
git commit -m "docs(simulator): add module rules for hydragrow-simulator"
```

### Task 3: Update global module rules and README

**Files:**
- Modify: `docs/superpowers/specs/module-rules/README.md`
- Modify: `README.md`

- [ ] **Step 1: Update docs/superpowers/specs/module-rules/README.md**

Locate the subsystem table and add a row for Simulator:
```markdown
| Simulator (`hydragrow-simulator/`) | [simulator.md](./simulator.md) | Sửa logic mô phỏng, test scenarios, giả lập hardware |
```
Locate the "Kiểm tra chung trước mọi PR" table and add the test command:
```markdown
`<br>(cd hydragrow-simulator && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test)`
```

- [ ] **Step 2: Update root README.md**

Add to Subsystems table:
```markdown
| Simulator | Rust | `hydragrow-simulator/` | N/A |
```
Add to CI table:
```markdown
| `simulator-ci` | push/PR chạm `hydragrow-simulator/`, `hydragrow-controller-core/` hoặc `hydragrow-shared/` | cargo fmt + check + clippy + test |
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/module-rules/README.md README.md
git commit -m "docs: add simulator to subsystem lists and rules"
```

### Task 4: Setup CI workflow for the simulator

**Files:**
- Create: `.github/workflows/simulator-ci.yml`

- [ ] **Step 1: Create the CI workflow file**

```yaml
name: Simulator CI

on:
  push:
    branches: [ main ]
    paths:
      - 'hydragrow-simulator/**'
      - 'hydragrow-controller-core/**'
      - 'hydragrow-shared/**'
      - '.github/workflows/simulator-ci.yml'
  pull_request:
    paths:
      - 'hydragrow-simulator/**'
      - 'hydragrow-controller-core/**'
      - 'hydragrow-shared/**'
      - '.github/workflows/simulator-ci.yml'

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Setup Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - name: Check format
        run: cd hydragrow-simulator && cargo fmt --check
      - name: Clippy
        run: cd hydragrow-simulator && cargo clippy --all-targets -- -D warnings
      - name: Test
        run: cd hydragrow-simulator && cargo test
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/simulator-ci.yml
git commit -m "ci(simulator): add github actions workflow for hydragrow-simulator"
```
