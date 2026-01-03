# Baseline Dataset Assets

## Purpose

Describe the structure and workflow for the baseline dataset used to seed new Praxis partitions:
where the data payloads live, how they are imported, and how changes are versioned and validated.

The files under `docs/data` describe the versioned baseline that seeds new Praxis datastores. They
are treated as **data-first** artefacts so changes stay reviewable, testable, and reproducible across
CI and local builds.

## Layout

- `meta/` - reference schema payloads used for validation/regression checks. Canonical schema
  definitions are owned by Praxis metamodel packages.
- `base/` - versioned strategy-to-execution datasets expressed as YAML. Each file contains an
  ordered set of operations/facts that seed elements, relationships, and artefacts with time/scenario
  context.

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

1. Update `base/baseline.yaml`, keeping operations append-only where possible. Use semantic versions
   in the `version` field and document the change in `CHANGELOG.md`.
2. Run the importer against a scratch datastore to validate:
   ```sh
   cargo aideon_xtask import-dataset --dataset docs/data/base/baseline.yaml \
       --datastore /tmp/praxis --dry-run
   ```
3. When satisfied, drop `--dry-run` (or point to a packaged datastore) to write:
   ```sh
   cargo aideon_xtask import-dataset --dataset docs/data/base/baseline.yaml \
       --datastore /tmp/praxis
   ```
4. Execute targeted dataset tests to ensure guardrail counts still match expectations.

## Quality gates

- YAML is validated via serde with strict schemas inside `praxis`.
- Importer dry-runs apply the dataset to an in-memory store so validation mirrors runtime behaviour.
- Baseline data remains versioned and deterministic for CI/regression coverage.
