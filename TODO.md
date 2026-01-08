# TODO

## Workflow

- `bindings` branch exists as worktree (`~/git/liana-bindings`)
- Stabilize core first, then fast-forward bindings to master and add APIs
- Avoid constant rebasing - core should be solid before bindings build on it

## Pending

- [ ] Stabilize core (`liana-core`, `liana-http`, `liana-codegen`)
- [ ] Multi-language codegen targets (Rust first, then TypeScript, Python, etc.)
- [ ] Fast-forward bindings branch to stable core
- [ ] Add first binding to dogfood the core

## Future Ideas

- Web APIs: fetch, WebSocket, etc.
- OpenAPI: GitHub, Stripe, etc.
- FFI: SQLite, etc.

## Backlog

- [ ] Branch-per-language for generated bindings (git as canonical source, registries as convenience)
  - `bindings`, `bindings-ts`, `bindings-py`, etc.
  - CI: schema changes → codegen → push to branch → publish to registry
  - Open question: branch per language vs per backend (e.g., ts-fetch vs ts-axios)
- [ ] Primary value is curated bindings (1000+ APIs), not just the codegen tool
  - Master has tooling, branches have the actual value

## Design Notes

### Module splitting
- Don't split by default - let consumers tree-shake
- If needed, split by logical grouping (OpenAPI tags, header files)

### Confidence-based generation
- Score confidence per binding during codegen
- High confidence: generate and trust
- Low confidence: generate but flag for review (comments/metadata)
- Better than refusing to generate complex cases

### Specialcases system
- Overrides on top of generated code (like Nix/portage patches)
- `schemas/<api>/specialcases.toml` - version controlled, auditable
- Supports: type overrides, signature fixes, doc comments, add/remove bindings
- Regeneration preserves and reapplies specialcases

### C namespace conventions → target language idioms
- C uses prefixes: `wlr_output_create()`, `xkb_keymap_new()`
- Target languages have proper namespacing: `Output::create()`, `Keymap::new()`
- Configurable strategies per binding:
  - Prefix stripping rules
  - Method vs static method detection (first arg = self?)
  - Module/namespace mapping
- Binding author specifies strategy in config

### Automation spectrum
- Fully automated: OpenAPI, simple FFI
- Assisted (heuristics + confidence): complex FFI
- 90% → 99% is the hard part, not 0% → 90%
