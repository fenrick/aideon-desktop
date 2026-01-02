# Domain Model

**Audience:** Engineers wiring Praxis Desktop to the digital twin. Captures the working nouns and how they relate.

## Core entities

- **Partition** - top-level workspace/tenant boundary for all data, schema, and artefacts.
- **Scenario** - overlay context for "what-if" changes; executed against a partition at a given time.
- **Time Context** - `{ valid_time, asserted_time?, layer }` applied to reads and writes.
- **Master Type** - stable structural anchor (Actor, Intent, Value, Capability, Execution, Technology, Structure, Change).
- **Domain Type** - extensible business concept inheriting from a master type.
- **Element** - a node instance of a domain type.
- **Relationship** - an edge instance of a domain verb mapped to master semantics.
- **Artefact** - a stored definition for **view**, **catalogue**, **matrix**, **map**, **report**, or **template**.
- **Task** - user-facing authoring operation (create element, link, set attribute, move/contain).
- **Widget** - UI block that renders an artefact result and emits selection.

## Relationships

- Partition **has many** Scenarios and Artefacts.
- Scenario **overlays** a Partition; tasks can write into a scenario context.
- Artefact **belongs to** a Partition and is executed with a Scenario + Time Context.
- Element **is typed by** a Domain Type; Domain Type **inherits from** a Master Type.
- Relationship **connects** Elements using a domain verb mapped to master semantics.
- Widgets **reference** Artefacts and render their results in the UI shell.

## Data flow

1. User selects **Scenario + Time Context** -> artefacts execute and return view/catalogue/matrix/map results.
2. User interacts with Widgets -> **Selection store** updates -> Inspector shows fields + actions.
3. User triggers **Tasks** (create/link/update) -> Praxis validates semantics -> Mneme stores facts/ops.
4. Artefacts re-run or invalidate caches based on changes, producing updated results.

## Source of truth

- **Mneme** stores bi-temporal facts, schema, and projection edges.
- **Praxis** owns meaning, metamodel packages, integrity rules, and task orchestration.
- Renderer persists only UI state; all domain changes flow through typed IPC to Praxis/Mneme.
