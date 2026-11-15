//! Adapter topics — detection order, configuration, and test entries.
//!
//! Responsibilities:
//! - pytest, cargo, and vitest detection and explicit `[runners]` configuration
//! - the contract test-entry format and failure normalization per adapter
//!
//! Rationale: adapter setup is a per-toolchain concern, and `missing-adapter` points here for the specifics

use super::super::model::{Hint, TopicEntry};

pub(crate) static ADAPTER_PYTEST: TopicEntry = TopicEntry {
    slug: "adapter-pytest",
    summary: "Python pytest adapter: detection, configuration, and test entry format.",
    body: "## adapter-pytest — Python pytest adapter

The pytest adapter runs Python tests via `pytest`. It is detected automatically
when any of the following are present (in priority order):

1. `runners.pytest` configured in `.espectacular/config.toml`
2. `pyproject.toml` with `[tool.pytest.ini_options]`
3. `pytest` executable on `PATH`
4. A `.py` file with `import pytest`

**Test entry format** (in contract `.toml`):
```toml
[tests]
pytest = [{flags = \"tests/test_module.py::TestClass::test_method\"}]
```

**Configuring explicitly**:
```toml
[runners]
pytest = [\"python\", \"-m\", \"pytest\"]
```

**Failure normalization**: exit code 1 → `test-failing`; exit code 2–5 →
`test-failing` with stderr captured; `ImportError` in stdout → `test-failing`
with import path hint.",
    when: "You are configuring or debugging the pytest adapter.",
    do_action: "Check detection order and add `runners.pytest` to config if needed.",
    human_approval: false,
    related_topics: &[
        "missing-adapter",
        "missing-runner",
        "adapter-cargo",
        "adapter-vitest",
    ],
    hints: &[Hint {
        kind: "tip",
        message: "Use `pyproject.toml` with `[tool.pytest.ini_options]` for automatic detection.",
    }],
};

pub(crate) static ADAPTER_CARGO: TopicEntry = TopicEntry {
    slug: "adapter-cargo",
    summary: "Rust cargo adapter: detection, configuration, and test entry format.",
    body: "## adapter-cargo — Rust cargo adapter

The cargo adapter runs Rust tests via `cargo test`. It is detected
automatically when `Cargo.toml` is present.

**Test entry format** (in contract `.toml`):
```toml
[tests]
cargo = [{flags = \"crate::module::tests::test_name\"}]
```

**Configuring explicitly**:
```toml
[runners]
cargo = [\"cargo\", \"test\", \"--\"]
```

**Failure normalization**: non-zero exit → `test-failing`; build errors are
classified as `test-failing` with the compiler message captured; test panics
include the test name and location.",
    when: "You are configuring or debugging the cargo adapter.",
    do_action: "Ensure `Cargo.toml` is present; add `runners.cargo` to config if needed.",
    human_approval: false,
    related_topics: &[
        "missing-adapter",
        "missing-runner",
        "adapter-pytest",
        "adapter-vitest",
    ],
    hints: &[],
};

pub(crate) static ADAPTER_VITEST: TopicEntry = TopicEntry {
    slug: "adapter-vitest",
    summary: "TypeScript vitest adapter: detection, configuration, and test entry format.",
    body: "## adapter-vitest — TypeScript vitest adapter

The vitest adapter runs TypeScript/JavaScript tests via `vitest`. It is
detected automatically when `package.json` contains a `vitest` dependency.

**Test entry format** (in contract `.toml`):
```toml
[tests]
vitest = [{flags = \"src/module.test.ts\"}]
```

**Configuring explicitly**:
```toml
[runners]
vitest = [\"npx\", \"vitest\", \"run\"]
```

**Failure normalization**: non-zero exit → `test-failing`; transform errors
(TypeScript compilation) are classified as `test-failing` with the error
location; import resolution failures include the unresolved module path.",
    when: "You are configuring or debugging the vitest adapter.",
    do_action: "Ensure `package.json` has a vitest dependency; configure runner if needed.",
    human_approval: false,
    related_topics: &[
        "missing-adapter",
        "missing-runner",
        "adapter-pytest",
        "adapter-cargo",
    ],
    hints: &[],
};
