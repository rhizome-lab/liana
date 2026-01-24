# CLAUDE.md

Behavioral rules for Claude Code in this repository.

## Architecture

**Branch structure:**
- `master` - core infrastructure only
- `bindings` - merges master + adds API bindings

**Directory structure (master):**
- `crates/` - core infrastructure (`concord-core`, `concord-http`, `concord-codegen`)
- `schemas/` - OpenAPI specs (fetched, not generated code)

**Directory structure (bindings branch adds):**
- `bindings/` - API bindings (`web-*`, `openapi-*`, `ffi-*`)

**Binding categories:**
- `web-*`: Browser/Web API bindings
- `openapi-*`: Generated from OpenAPI schemas
- `ffi-*`: Foreign function interface bindings

**OpenAPI workflow:** Schemas stored in `schemas/<api-name>/`. Generation via `cargo run -p rhi-concord-codegen`. Generated code is a build artifact, not checked into git.

**Multi-language:** Codegen targets multiple languages (Rust, TypeScript, Python, etc.). Consumers use native bindings without needing Rust/cargo. Schemas are the universal source of truth.

## Core Rule

**Note things down immediately:**
- Bugs/issues → fix or add to TODO.md
- Design decisions → docs/ or code comments
- Future work → TODO.md
- Key insights → this file

**Triggers:** User corrects you, 2+ failed attempts, "aha" moment, framework quirk discovered → document before proceeding.

**Don't say these (edit first):** "Fair point", "Should have", "That should go in X" → edit the file BEFORE responding.

**Do the work properly.** When asked to analyze X, actually read X - don't synthesize from conversation.

**If citing CLAUDE.md after failing:** The file failed its purpose. Adjust it to actually prevent the failure.

## Behavioral Patterns

From ecosystem-wide session analysis:

- **Question scope early:** Before implementing, ask whether it belongs in this crate/module
- **Check consistency:** Look at how similar things are done elsewhere in the codebase
- **Implement fully:** No silent arbitrary caps, incomplete pagination, or unexposed trait methods
- **Name for purpose:** Avoid names that describe one consumer
- **Verify before stating:** Don't assert API behavior or codebase facts without checking

## Commit Convention

Use conventional commits: `type(scope): message`

Types:
- `feat` - New feature
- `fix` - Bug fix
- `refactor` - Code change that neither fixes a bug nor adds a feature
- `docs` - Documentation only
- `chore` - Maintenance (deps, CI, etc.)
- `test` - Adding or updating tests

Scope is optional but recommended for multi-crate repos.

## Negative Constraints

Do not:
- Announce actions ("I will now...") - just do them
- Leave work uncommitted
- Create special cases - design to avoid them
- Create legacy APIs - one API, update all callers
- Do half measures - migrate ALL callers when adding abstraction
- Ask permission when philosophy is clear - just do it
- Return tuples - use structs with named fields
- Use trait default implementations - explicit impl required
- Replace content when editing lists - extend, don't replace
- Cut corners with fallbacks - implement properly for each case
- Mark as done prematurely - note what remains
- Fear "over-modularization" - 100 lines is fine for a module
- Consider time constraints - optimize for correctness
- Use path dependencies in Cargo.toml - causes clippy to stash changes across repos
- Use `--no-verify` - fix the issue or fix the hook
- Assume tools are missing - check if `nix develop` is available for the right environment

## Design Principles

**Backend agnostic:** Bindings should work with any async runtime (tokio, async-std, smol) and HTTP client. Use traits for abstraction. Feature flags for optional backends.

**Edition 2024:** Use Rust edition 2024 features. Embrace new patterns (e.g., async closures, gen blocks where applicable).

**Type safety over convenience:** Prefer strongly-typed APIs. Use newtypes for IDs, enums for finite sets. Avoid stringly-typed interfaces.

**Unify, don't multiply.** One interface for multiple cases > separate interfaces. Plugin systems > hardcoded switches.

**Simplicity over cleverness.** HashMap > inventory crate. OnceLock > lazy_static. Functions > traits until you need the trait.

**Explicit over implicit.** Log when skipping. Show what's at stake before refusing.

**When stuck (2+ attempts):** Step back. Am I solving the right problem?
