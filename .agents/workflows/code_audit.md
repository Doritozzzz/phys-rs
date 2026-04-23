---
description: Workflow to audit the existing codebase for compliance with architecture, numerical stability, and physics formula rules.
---

# Workflow: Code Audit & Technical Debt Review

Use this workflow to ensure that the codebase remains healthy, safe, and aligned with the engine's engineering standards. This should be performed periodically or before major releases.

## Step 1: Structural Audit

1. **ECS Pattern Check**: Verify that `src/components/` only contains data and `src/physics/` only contains logic.
2. **Module Hygiene**: Ensure all directories have a `mod.rs` and that re-exports are clean.
3. **Dependency Check**: Ensure no unauthorized dependencies have been added to `Cargo.toml`.

## Step 2: Numerical Safety Audit

1. **Precision Sweep**: Search for any usage of `f32` in physics-sensitive areas (anything outside `render/`).
   - // turbo
     ```bash
     grep -r "f32" src/physics src/core src/components
     ```
2. **Softening Verification**: Check all division operations in `src/physics/`. Ensure every inverse-distance calculation includes a softening factor.
3. **Finite Check Audit**: Ensure all systems that update `Position`, `Velocity`, or `Force` components contain `debug_assert!(value.is_finite())` or `assert!(value.is_finite())`.

## Step 3: Physics Compliance Audit

1. **Master Index Sync**: For every system in `src/physics/`, verify that:
   - The doc comment references the correct section of `docs/PHYSICS_MASTER_INDEX.md`.
   - The implemented formula matches the LaTeX in the index exactly.
   - Units are consistent and documented.

## Step 4: Documentation & Backlog Audit

1. **Completeness**: Check that every public struct/function has a `///` doc comment.
2. **Backlog Consistency**: Verify that everything marked `[x]` in `docs/BACKLOG.md` is actually present and functional in the codebase.
3. **Roadmap Alignment**: Check if the current implementation matches the goals set in `docs/ROADMAP.md`.

## Step 5: Performance & Tests

1. **Unit Tests**: Run all tests to ensure no regressions.
   - // turbo
     ```bash
     cargo test
     ```
2. **Clippy & Lints**: Run clippy for idiomatic Rust checks.
   - // turbo
     ```bash
     cargo clippy -- -D warnings
     ```

## Step 6: Generate Audit Report

Create an artifact summarizing:
- Found issues (Critical vs. Minor).
- Compliance score (0-100%).
- Recommended actions to be added to the `BACKLOG.md`.
