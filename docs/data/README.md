# Baseline Dataset Assets

## Purpose

Describe the structure and workflow for the baseline dataset used to seed new Praxis datastores:
where the YAML and schema payloads live, how they are imported, and how changes are versioned and
validated.

The files under `docs/data` describe the versioned baseline that seeds every new Praxis datastore.
They are treated as **data first** artifacts so changes stay reviewable, testable, and reproducible
across CI and local builds.

## Layout

- `meta/` – canonical schema payloads (e.g., `core-v1.json`). These are embedded
  at build time so the host and importer always use the same validation rules.
- `base/` – versioned strategy-to-execution datasets expressed as YAML. Each
  file contains one or more commits that the importer replays onto the `main`
  branch so the renderer immediately has realistic graph data, plan events, and
  cross-domain links.

```
docs/data/
  README.md
  meta/
    core-v1.json
  base/
    baseline.yaml         # primary dataset
    CHANGELOG.md          # short history with semantic versions
```

## Editing workflow

1. Update `base/baseline.yaml`, keeping commits append-only. Use semantic
   versions in the `version` field and document the change in `CHANGELOG.md`.
2. Run the importer against a scratch datastore to validate:
   ```sh
   cargo aideon_xtask import-dataset --dataset docs/data/base/baseline.yaml \
       --datastore /tmp/praxis --dry-run
   ```
3. When satisfied, drop `--dry-run` (or point to a packaged datastore) to write
   the commits:
   ```sh
   cargo aideon_xtask import-dataset --dataset docs/data/base/baseline.yaml \
       --datastore /tmp/praxis
   ```
4. Execute `cargo test -p aideon-praxis dataset::tests::baseline_counts` to
   ensure guardrail counts still match expectations.

## Quality gates

- YAML is validated via serde with strict schemas inside `aideon_praxis`.
- Every commit carries `baseline` tags so downstream tooling can distinguish
  imported history from user edits.
- Importer dry-runs apply the dataset to an in-memory engine so attribute and
  relationship validation matches runtime behavior.
