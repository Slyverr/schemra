# Agent guidance

- Read [README.md](README.md) for the project overview, requirements, and how to
  run the CLI and Studio.
- Follow [CONTRIBUTING.md](CONTRIBUTING.md) for change workflow, test placement,
  validation commands, fixtures, and commit conventions.
- Use relevant project-local guidance in `.agents/skills/` when working in
  Rust, Svelte, or Playwright. Skills guide implementation; they are not runtime
  dependencies.
- For Studio work, keep layouts and interactions consistent with the product.
  Use short, cohesive modules with focused components and shallow control flow;
  split by responsibility, not just to reduce line counts.
- Follow the shared Studio UI component rule in `CONTRIBUTING.md` for controls,
  menus, and dialogs.
- Describe Schemra as a standalone product. Keep project files free of
  provenance and comparison notes.
- Preserve unrelated staged and unstaged work. Do not change the Git index or
  create commits unless the user asks.
