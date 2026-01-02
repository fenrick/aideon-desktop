# Aideon Praxis Design Spec (Task-Oriented Interaction + Fully Flexible Metamodel)

## 1. Purpose

Praxis is the **enterprise modelling and reasoning layer** that gives “digital twin” power without exposing graph or storage complexity.

Praxis provides:

- A **stable conceptual backbone** (master types + master relationship semantics)
- **Unlimited extensibility** via domain types (inheritance, defaults, rules)
- A **task-oriented interaction model** that stays consistent even as the metamodel grows
- A **quality and integrity system** that keeps analytics meaningful (PageRank, dependency risk, change impact)
- A strict separation where **Mneme owns persistence** (SeaORM/SQLx) and **Praxis owns meaning**

Praxis never generates SQL and never exposes storage mechanics.

---

## 2. Core design principle (the “digital twin” simplification)

Praxis reconciles three different needs:

1. **Users think in tasks**
   “Create a journey”, “link service to capability”, “compare scenarios”.

2. **Architects think in concepts**
   Service, capability, application module, release train, contract, control, etc.

3. **Analytics needs stable structure**
   Consistent directionality, repeatable patterns, minimal semantic ambiguity.

Praxis achieves this with a **three-layer model**:

- **Layer A: Master types (small, stable, structural anchors)**
- **Layer B: Domain types (large, extensible, user/business concepts)**
- **Layer C: Tasks (UI/API operations that work across many domain types)**

Tasks operate on **master type roles**, not on individual domain types.

---

## 3. System boundaries and dependencies

### 3.1 Depends on Mneme

Praxis uses Mneme APIs for:

- entity creation (nodes/edges)
- properties and time-valid facts
- scenario overlays
- traversal at time
- analytics projection edges
- export/import (op-log, snapshots)

### 3.2 Provides to the rest of Aideon

- metamodel packages (default + extensions)
- domain authoring APIs (task-oriented)
- view definitions (canonical subgraphs)
- integrity/validation reports
- analytics orchestration and explainability

---

## 4. Praxis conceptual type system

### 4.1 Master types (anchors)

Master types are **few** and **immutable**. They define _structural role_ in the digital twin.

Each domain type must inherit from exactly one master type.

#### 4.1.1 Master node types

1. **Actor**
   - parties who act or are acted upon
     Examples: customer, persona, role, team, vendor, regulator

2. **Intent**
   - motivations and evaluative drivers
     Examples: goal, objective, pain, gain, driver, influence, brand attribute, principle

3. **Value**
   - value propositions and value flow constructs
     Examples: product, service, journey, touchpoint, feature, outcome, value stream

4. **Capability**
   - stable abilities the enterprise needs
     Examples: business capability, enabling capability, operational capability

5. **Execution**
   - operational realisation mechanisms
     Examples: process, operating model, control, policy, information asset, conceptual model

6. **Technology**
   - technology landscape at logical and physical levels
     Examples: logical app, module, API, batch interface, physical app, platform, infrastructure, logical tech

7. **Structure**
   - organisational and structural containers
     Examples: org unit, location, portfolio structure, contracts container, domain boundaries

8. **Change**
   - change over time and delivery constructs
     Examples: project, programme, portfolio, plateau, release train, roadmap milestone

> Note: “Service above Capability” is implemented as a **default domain model** under these anchors (Value → Capability), not by making Service a master type.

#### 4.1.2 Master edge semantics (relationship anchors)

Master edges define the **meaning and direction** required for analytics and integrity.

1. **Influences** (Intent/Actor → Intent/Value/Capability/Change)
   “A influences B” (incoming = depends-on influence)

2. **Realises** (Value → Capability)
   “Value proposition is realised by capability”

3. **IsRealisedBy** (Capability → Execution)
   “Capability is realised by execution”

4. **Enables** (Execution/Technology/Structure → Execution/Capability/Value)
   “A enables B” (direction = dependency)

5. **Implements** (Technology logical → Technology physical)
   “Logical concept implemented by physical”

6. **DependsOn** (Any → Any)
   Generic dependency when no stronger verb applies (discouraged in default modelling)

7. **BelongsTo** (Any → Structure/Value/Change container)
   Membership/containment (used for modules in platforms, features in products, etc.)

8. **Changes** (Change → Any)
   Planned or executed change affecting target(s)

9. **Consumes/Produces** (Execution/Technology ↔ Information assets)
   Data and info flow (direction matters for lineage and analytics variants)

Praxis may offer domain-friendly verbs that map to these anchors.

---

## 5. Domain types (extensible)

### 5.1 Definition

Domain types are the actual concepts your users care about:

- Customer, Persona, Journey, Touchpoint
- Service, Product, Feature
- Pain, Gain, Driver, Objective, Brand Attribute
- Capability, Value Stream
- Role, Team, Org Unit, Location
- Operating Model, Process, Control
- Information Asset, Conceptual Data Model, Data Entity
- API, Batch Interface, App Communication
- Logical App, App Function, Module, Microservice
- Physical App, Platform, Physical Tech
- Vendor, Contract
- Project, Programme, Portfolio
- Plateau, Release Train, Roadmap, Milestone

### 5.2 Domain type rules

Each domain type must:

- inherit from exactly one master node type
- optionally inherit from another domain type (single inheritance)
- declare allowed relationship verbs (domain) that map to master edge semantics
- declare default fields and constraints

### 5.3 Single inheritance only

Single inheritance is mandatory:

- avoids ambiguous defaults
- simplifies explainability
- keeps analytics behaviour stable
- aligns with how end users understand “nested inheritance”

---

## 6. Metamodel governance model

Praxis separates “who can change the language of the twin” from “who can use the language”.

### 6.1 Roles

- **Metamodel Administrator**
  - can create/edit domain types, fields, and rules
  - can publish metamodel packages to partitions

- **Model Author**
  - can create instances and links within allowed patterns
  - cannot create types/fields unless granted

- **Consumer**
  - read-only, runs views and analytics

### 6.2 Package-based evolution

Metamodel changes are versioned and published as **packages**:

- `aideon_core_twin` (mandatory baseline)
- `customer_experience`
- `product_and_service_design`
- `controls_and_risk`
- `delivery_and_change`
- `data_and_integration`
- etc.

Packages can be installed per partition.

---

## 7. Storage representation contract (Praxis → Mneme)

Praxis expresses metamodel and all modelling operations using Mneme’s types/fields/edges APIs.

### 7.1 Stable IDs

All master types, domain types, fields, and edge types use **stable UUIDs** committed in source.
No regeneration allowed.

### 7.2 Praxis metamodel compilation output

Praxis produces a Mneme `MetamodelBatch` with:

- Types (node/edge)
- Fields (typed)
- TypeFields (required/defaults/overrides)
- EdgeTypeRules (semantic direction + endpoint constraints)

Praxis must also store its own **domain registry** (in code and/or as Mneme records) mapping:

- DomainTypeKey → Mneme type_id
- DomainFieldKey → Mneme field_id
- DomainVerb → Mneme edge_type_id + direction

---

## 8. The task-oriented interaction model (what users see)

### 8.1 What “task-oriented” means

Praxis exposes tasks that:

- operate across multiple domain types
- enforce rules and direction automatically
- hide graph mechanics (nodes/edges/fields IDs)
- remain stable even if the metamodel expands

Task orientation is **not** a restriction; it is an interaction contract.

### 8.2 Task categories

1. **Create and manage elements**
   - create element of a domain kind
   - rename, describe, classify
   - attach owner, criticality, maturity, etc.

2. **Create and manage relationships**
   - link using domain verb (“realises”, “enabled by”, “implements”, “part of”)
   - Praxis determines direction and validates endpoints

3. **Time-based changes**
   - set effective dates
   - set future plateaus and backdated corrections
   - plan vs actual (layer)

4. **Scenarios**
   - create scenario
   - change inside scenario
   - compare to baseline
   - promote (optional, defined later)

5. **Views**
   - run canonical views (strategy-to-execution, service dependency, capability risk)
   - get subgraph results

6. **Analytics**
   - get “criticality” rankings (PageRank-style)
   - get change impact and blast radius
   - explain results

---

## 9. Praxis master contracts for analytics validity

To keep PageRank and diagnostics meaningful, Praxis must enforce (at minimum):

### 9.1 Directionality integrity

All edges must have unambiguous dependency direction consistent with master semantics.

### 9.2 Logical/physical separation

- Capabilities connect to **logical** apps and tech concepts
- Physical apps/tech never connect directly to capabilities (unless a rule explicitly allows it and analytics labels it as a shortcut)

### 9.3 End-to-end connectivity expectations

Praxis defines required “connectivity spines” that analytics assumes, for example:

- Intent → Value → Capability → Execution → Technology (Logical → Physical)

Missing links are not just “incomplete”; they degrade analytics confidence and must be reported.

---

## 10. Praxis internal subsystems

1. **Domain Registry**
   - authoritative mapping of domain concepts to Mneme IDs
   - installed package list per partition
   - provides lookups for tasks

2. **Metamodel Publisher**
   - builds Mneme metamodel batches from packages
   - applies to partitions
   - triggers Mneme processing (effective schema compilation, integrity refresh)

3. **Rule Engine**
   - evaluates endpoint constraints, required patterns, forbidden shortcuts
   - produces domain-language findings

4. **Authoring Service**
   - implements tasks (create element, link, set attribute, etc.)
   - translates to Mneme writes with time/scenario context

5. **View Service**
   - canonical, named view definitions
   - consistent time/scenario filtering
   - bounded traversal and pagination

6. **Analytics Orchestrator**
   - requests adjacency from Mneme projection edges
   - runs algorithms (or delegates)
   - stores results (optional)
   - provides explainability paths

7. **Explainability Engine**
   - explains:
     - why a node ranks highly
     - which dependencies contribute
     - what changed between scenarios

---

## 11. Context model (time/scenario/user)

Praxis requires an explicit context on all operations:

- `partition_id`
- `scenario_id` (optional)
- `actor_id`
- `asserted_at` (generated or supplied)
- `valid_from`, `valid_to` (optional)
- `layer` (Plan/Actual)

Praxis defaults:

- `valid_from = now`
- `layer = Actual`
- scenario = none (baseline)

---

## 12. Detailed metamodel specification approach (how Praxis covers the full breadth)

Praxis does not create one enormous unstructured model. It provides:

- master anchors (Section 4)
- domain packages grouped by area

### 12.1 Required domain package groups (evergreen)

1. **Customer and Experience**

- Customer, Persona, Segment
- Journey, Stage, Touchpoint, Channel
- Need, Pain, Gain, Outcome
- Experience Measure (NPS, CSAT, etc.)

2. **Product and Service Design**

- Product, Offering, Service
- Feature, Epic, Requirement (optional)
- Service Blueprint concepts (optional)
- Value Proposition / Benefit

3. **Business Motivation and Strategy**

- Vision, Goal, Objective
- Driver, Influence
- Constraint, Principle
- Brand Attribute / Promise

4. **Capabilities and Value Delivery**

- Business Capability (with level hierarchy)
- Value Stream
- Service Catalogue linkage
- Capability investment attributes

5. **Organisation and Operating Model**

- Role, Team, Org Unit
- Location, Site
- Operating Model
- Process, Procedure
- KPI/Measure
- Control, Policy

6. **Information, Data, and Integration**

- Information Asset
- Conceptual Data Model, Data Entity
- Data Domain
- Data Quality / Classification
- Interface: API, Event, Batch, File Transfer
- Application Communication patterns

7. **Application and Technology**

- Logical Application
- Application Function
- Module / Component / Service
- Physical Application
- Technology Platform
- Physical Technology / Infrastructure
- Environments (optional)
- Vendor / Product taxonomy

8. **Risk, Controls, and Compliance**

- Control objectives
- Controls
- Risks (optional)
- Obligations (optional)

9. **Change and Delivery**

- Portfolio, Programme, Project
- Initiative
- Plateau (baseline and target states)
- Roadmap, Milestone
- Release Train
- Deployment Unit (optional)

These packages can be installed progressively.

---

## 13. Relationship model (explicit mapping)

### 13.1 Domain verbs map to master semantics

Examples:

- “Journey **contains** Touchpoint” → BelongsTo
- “Service **realises** Capability” → Realises
- “Capability **is realised by** Process” → IsRealisedBy
- “Process **is enabled by** Logical App” → Enables
- “Logical App **is implemented by** Physical App” → Implements
- “API **belongs to** Application Module” → BelongsTo
- “Project **changes** Application” → Changes
- “Driver **influences** Goal” → Influences

### 13.2 Endpoint constraints (default)

Praxis defines constraints at the master layer, then refines per domain type where needed.

Example constraints:

- Realises: Value → Capability only
- Implements: Technology(logical) → Technology(physical) only
- IsRealisedBy: Capability → Execution only

These are encoded into Mneme `edge_type_rules` for portability and for analytics consistency.

---

## 14. Views (canonical exploration)

Praxis provides default views that are stable, named, and task-friendly:

1. Strategy-to-Execution Spine

- Intent → Value → Capability → Execution → Technology

2. Service Dependency View

- Services, their realised capabilities, supporting logical apps, implementing physical apps

3. Capability Risk Concentration

- Capabilities ranked by dependency concentration, with application/platform hotspots

4. Customer-to-Capability Trace

- Journey → touchpoints → services/products → capabilities → execution

5. Change Impact View

- Plateau/Release Train/Programme → impacted services/capabilities/apps

Views are bounded, paginated, and time/scenario aware.

---

## 15. Analytics model (user-facing, explainable)

Praxis exposes analytics as domain questions:

- “Most critical capabilities”
- “Services with highest dependency risk”
- “Applications with highest blast radius”
- “What changes most between baseline and scenario?”

Praxis runs analytics on Mneme-provided projection edges filtered by view and time.

Explainability is mandatory:

- top contributing inbound dependencies
- top paths from anchor concepts (services/capabilities) to the ranked node
- difference explanations between scenarios (delta in key inbound edges)

---

## 16. Integrity and quality (minimum gates)

Praxis maintains an integrity score per partition/scenario/time slice.

### 16.1 Minimum gates before analytics

Analytics should be gated by:

- directionality compliance above threshold
- logical/physical separation violations below threshold
- connectivity spine completeness above threshold

If thresholds not met:

- allow analytics with warning and reduced confidence
- or block (configurable)

---

## 17. Public APIs (Rust)

### 17.1 Metamodel management

- list packages
- install/upgrade packages to a partition
- validate package consistency
- publish to Mneme

### 17.2 Authoring (task APIs)

- create element (domain kind)
- link elements (domain verb)
- set attribute(s)
- move/contain (BelongsTo)
- delete/tombstone (soft)

### 17.3 Scenario and time

- create scenario
- compare scenario to baseline
- time-slice reads

### 17.4 Views and analytics

- run view
- run ranking
- explain ranking
- run integrity scan and get findings

Praxis never exposes Mneme IDs directly unless required for interoperability; instead, it exposes stable domain IDs that map 1:1 to Mneme entity IDs.

---

## 18. Public APIs (TypeScript)

TypeScript APIs must be:

- task-oriented (same semantics as Rust)
- stable across metamodel expansion
- explicit about time/scenario context

Example TS tasks:

- `createJourney`, `createService`, `createCapability`, `createLogicalApp`, etc. as convenience wrappers around a generic `createElement(kind)`
- `link({verb, src, dst})`
- `setEffectiveAttribute({validFrom, layer})`
- `runView(viewId)`
- `getCriticalCapabilities()`
- `compareScenarioImpact()`

---

## 19. Testing requirements (Praxis)

1. Package correctness tests

- stable IDs
- inheritance acyclic
- required fields exist
- relationship verbs map to master semantics

2. Integrity enforcement tests

- invalid shortcuts flagged
- direction violations flagged
- required spine gaps detected

3. Cross-DB behavioural parity (through Mneme)

- same modelling operations yield same query results on SQLite/Postgres/MySQL

4. Scenario/time tests

- future plateaus, backdating
- scenario overlay compare
- explainability outputs stable

---

## 20. Deliverables (modules)

- `praxis_registry` (domain registry, IDs, package loader)
- `praxis_metamodel` (package definitions, publisher)
- `praxis_rules` (rule templates, validator, findings)
- `praxis_authoring` (task services)
- `praxis_views` (view definitions + execution)
- `praxis_analytics` (orchestration + explainability)
- `praxis_ts` (bindings)

---

## 21. Explicit answers to the key questions

### 21.1 Does task orientation limit metamodel flexibility?

No. Tasks target **master type roles** and **domain verbs**, so new domain types are naturally supported as long as they inherit correctly and declare mappings.

### 21.2 Are there defined master types?

Yes. Master types are required. They are the structural anchors that make:

- modelling coherent
- time/scenario manageable
- analytics meaningful
- the user experience simple

### 21.3 How does Praxis cover the breadth of enterprise concepts listed?

By:

- a small immutable master backbone
- many domain packages spanning CX → product/service → motivation → capability → execution → data/integration → technology → change
- strict mapping of domain concepts and verbs to master roles and master semantics

This is the simplification that enables a usable, extensible digital twin rather than an ungoverned graph.

## 22. User-facing artefacts (views, matrices, catalogues, maps)

Praxis must let users work with the model through a small set of **high-leverage artefacts**. These artefacts are the “work products” of modelling and analysis.

Praxis artefacts are:

- definable by admins (and optionally power users)
- stored and versioned
- runnable at a valid time + scenario
- visualisable consistently across UIs

Artefact types:

1. **Views** (graph subgraphs and narratives)
2. **Catalogues** (lists with filters, grouping, and drill-down)
3. **Matrices** (2D relationships with counts and coverage)
4. **Maps** (layered diagrams: capability maps, journey maps, app landscapes)
5. **Reports** (packaged outputs with metrics, rankings, findings)

---

## 23. Custom views: definition, storage, execution, visualisation

### 23.1 View definition model (explicit)

A Praxis view is a declarative specification that defines:

- **Scope**
  - which domain/master types are in scope
  - seed selection rules

- **Traversal**
  - allowed relationship verbs (mapped to master semantics)
  - direction and depth
  - stop conditions

- **Filters**
  - time, scenario, layer
  - property filters
  - integrity gates

- **Projection**
  - which fields to include on nodes/edges
  - aggregation rules (counts, rollups)

- **Visual defaults**
  - layout hints
  - grouping and colouring rules (semantic categories)
  - labels and icons (UI layer uses this)

Views must be definable without writing code.

### 23.2 View storage (Praxis-owned; persisted via Mneme)

Views are stored as domain artefacts in Mneme as nodes + properties, with versioning.

#### 23.2.1 Storage representation

- `View` is a domain type under master `Structure` (or `Value` if treated as a “product of analysis”). Recommended: `Structure`.
- Each view has:
  - stable `view_id`
  - JSON definition blob (portable) stored as a property
  - metadata fields for indexing (name, tags, owner, scope types)

#### 23.2.2 Required view fields

- `name` (indexed)
- `description`
- `artefact_kind = "view"`
- `definition_json` (text)
- `created_by`, `updated_by`
- `visibility` (private/team/partition/public)
- `tags` (optional OR-Set semantics)

#### 23.2.3 Versioning model

Views are versioned as time-valid properties:

- `definition_json` can change over time with valid intervals
- optionally store a `definition_hash` for cache keys

This enables:

- “view as-of last quarter”
- controlled evolution without breaking bookmarks

### 23.3 View execution pipeline

Inputs:

- `partition_id`, optional `scenario_id`
- `at_valid_time`
- view definition (resolved at time)
- optional seeds (selected entities)

Steps:

1. Resolve view definition at time T
2. Resolve seed set:
   - direct list
   - query-based selection (catalogue filters)

3. Execute bounded traversal:
   - use Mneme traversal at time
   - enforce allowed edge types/verbs
   - apply stop rules

4. Enrich subgraph with requested fields:
   - batch `read_entity_at_time` calls (with field projection)

5. Apply projection rules:
   - group nodes/edges
   - compute rollups (counts, coverage)

6. Return a **ViewResult** with:
   - nodes, edges
   - grouped collections
   - computed summary metrics
   - integrity warnings if applicable

### 23.4 ViewResult schema (UI-ready)

```json
{
  "view_id": "…",
  "at_valid_time": "…",
  "scenario_id": null,
  "summary": {
    "node_count": 123,
    "edge_count": 456,
    "integrity_score": 0.82,
    "warnings": [ ... ]
  },
  "nodes": [
    {
      "id": "…",
      "kind": "BusinessCapability",
      "label": "Payments",
      "fields": { "owner": "...", "criticality": 4 },
      "groups": ["Capability:L2:Customer"]
    }
  ],
  "edges": [
    {
      "id": "…",
      "verb": "Realises",
      "src": "…",
      "dst": "…",
      "weight": 1.0,
      "fields": {}
    }
  ],
  "groups": [
    { "group_id": "Capability:L2:Customer", "title": "Customer Capabilities", "node_ids": ["…"] }
  ],
  "layouts": {
    "recommended": "layered",
    "hints": { "layers": ["Intent","Value","Capability","Execution","Technology"] }
  }
}
```

### 23.5 Visualisation defaults (portable spec)

Praxis stores **visual hints** in view definitions, but rendering is done by UI modules.

Supported layout hints:

- `layered` (master spine ordering)
- `swimlane` (journey stages/touchpoints)
- `hierarchy` (capability map)
- `force` (generic graph)
- `matrix` (delegated to matrix artefact)

Visual grouping rules (declarative):

- group by master type
- group by domain type
- group by taxonomy fields (e.g., capability level)
- group by ownership

---

## 24. Catalogues: lists with filters, groupings, and drill-down

### 24.1 Catalogue definition

A catalogue is a reusable list definition:

- target type(s)
- filters and search fields
- columns to display
- grouping and sorting rules
- optional “coverage” metrics

Users interact with catalogues constantly (service catalogue, app catalogue, capability catalogue).

### 24.2 Catalogue storage

Catalogue is an artefact type stored like views:

- `artefact_kind = "catalogue"`
- `definition_json`
- indexed fields: name, target kinds, tags

### 24.3 Catalogue execution

Praxis translates catalogue definition into:

- Mneme `list_entities` with indexed filters
- then batch enrich fields
- then group and aggregate in Praxis

### 24.4 Catalogue definition schema (example)

```json
{
  "target_kinds": ["BusinessService"],
  "filters": [{ "field": "status", "op": "Eq", "value": "Active" }],
  "columns": [
    { "field": "name" },
    { "field": "owner" },
    { "field": "criticality" },
    { "field": "lifecycle" }
  ],
  "group_by": ["domain", "lifecycle"],
  "sort": [{ "field": "criticality", "dir": "desc" }]
}
```

### 24.5 Catalogue result schema

- rows with stable IDs
- group headers with counts
- coverage stats when requested:
  - % services linked to at least one capability
  - % apps mapped to logical layer
  - etc.

---

## 25. Matrices: coverage and relationship analysis

Matrices are essential for EA users:

- capability × application coverage
- service × capability mapping
- value stream × capability
- process × information asset
- app × data entity
- change initiative × impacted assets

### 25.1 Matrix artefact definition

A matrix definition specifies:

- axis X: a catalogue definition (type(s) + filters)
- axis Y: a catalogue definition
- relationship predicate: which verbs/edge types count as “connection”
- aggregation:
  - boolean presence
  - count of links
  - weighted score

- cell drill-down:
  - list of connecting edges
  - list of paths (optional)

### 25.2 Matrix storage

- `artefact_kind = "matrix"`
- `definition_json`
- indexes: axis types, name, tags

### 25.3 Matrix execution pipeline

1. Resolve axis sets X and Y at time/scenario
2. Fetch relevant edges:
   - either:
     - query projection edges from Mneme filtered by edge types and endpoints
     - or traverse per-row (slower; avoid)

3. Build cell aggregation in memory
4. Return:
   - matrix grid (sparse representation recommended)
   - row/column totals
   - coverage measures
   - drill-down references

### 25.4 Matrix result format (sparse)

```json
{
  "rows": [{ "id": "cap1", "label": "Payments" }],
  "cols": [{ "id": "app7", "label": "Core Banking" }],
  "cells": [{ "r": "cap1", "c": "app7", "v": 3, "edge_ids": ["e1", "e2", "e3"] }],
  "row_totals": [{ "id": "cap1", "total": 12 }],
  "col_totals": [{ "id": "app7", "total": 98 }]
}
```

### 25.5 Coverage metrics (built-in)

Praxis must support common EA coverage metrics as first-class:

- coverage % (rows with ≥1 connection)
- density (cells populated / total possible)
- hotspots (highest-degree rows/cols)
- gaps (rows/cols with zero)

These feed integrity and prioritisation.

---

## 26. Maps: structured visual models (capability maps, journeys, landscapes)

Maps are curated visualisations that are not “generic graphs”.

Map types:

1. **Capability Map**
2. **Journey Map**
3. **Application Landscape**
4. **Operating Model Map**
5. **Technology Platform Map**
6. **Roadmap / Plateau Map**

### 26.1 Map artefact definition

A map definition specifies:

- target domain/master types
- hierarchy rules or lanes
- layout model
- display fields
- optional overlays (metrics, risk, maturity, PageRank scores)

### 26.2 Map storage

- `artefact_kind = "map"`
- `map_kind` field (capability/journey/app-landscape/etc.)
- `definition_json`

### 26.3 Capability map specifics

Capability map requires:

- capability hierarchy fields (L1/L2/L3)
- ownership or domain grouping
- overlays:
  - criticality
  - investment
  - dependency risk (from PageRank or degree stats)
  - change initiatives affecting each capability

Praxis must provide a standard capability map view that can be customised.

### 26.4 Journey map specifics

Journey map requires:

- journey stages
- touchpoints
- related services/products
- pains/gains/outcomes per stage
- optional metrics (drop-off, satisfaction)

These are domain types under master Value/Intent and are mapped into lanes and sequences.

### 26.5 Application landscape specifics

Landscape requires:

- logical apps grouped by domain/capability
- physical apps/platforms under them
- integration edges (API/event/batch)
- vendor/platform overlays

---

## 27. Artefact registry and discovery

Users need to find and reuse artefacts.

### 27.1 Artefact catalogue (meta-catalogue)

Praxis provides a built-in catalogue over artefacts:

- views
- matrices
- maps
- reports

Filterable by:

- tags
- owner
- target domain types
- last updated
- visibility

### 27.2 Permissions and visibility

Artefacts have `visibility`:

- private
- team
- partition
- public (optional)

Praxis enforces at API level.

---

## 28. Artefact composition (assembling outputs)

Users need compositions like:

- “Capability Map with PageRank overlay, filtered to domain X”
- “Matrix + drill-down view + report”

Praxis supports **Artefact Templates**:

- a template references other artefacts
- provides parameter bindings
- outputs a composite result structure

### 28.1 Template artefact

- `artefact_kind = "template"`
- `definition_json` includes:
  - components list
  - parameter schema
  - default parameters

---

## 29. Parameterisation (time, scenario, filters as first-class)

All artefacts support parameters:

- `at_valid_time`
- `scenario_id`
- domain filters (taxonomy fields, owners, lifecycle)
- scope seeds (start from this service/capability)

Artefact definitions must specify:

- parameter schema
- defaults
- which parameters affect caching

---

## 30. Caching strategy (Praxis-level)

Praxis should cache results for:

- expensive matrices
- complex views
- maps with overlays

Caching keys include:

- partition_id
- scenario_id
- at_valid_time
- artefact_id + definition_hash
- filter params hash
- (optional) as_of_asserted_at for audit mode

Caches are ephemeral (memory/disk) and invalidated by:

- Mneme change feed events (op IDs after last run)
- artefact definition changes

Praxis must never treat caches as authoritative.

---

## 31. Drill-down and explainability (across all artefacts)

Every artefact must support drill-down:

- click a cell → show connecting edges and underlying elements
- click a hotspot → show inbound/outbound dependencies
- click a missing coverage gap → suggest next modelling actions

Praxis provides a common drill-down API:

- given a set of node IDs and/or edge IDs, return:
  - element summaries
  - relationship summaries
  - time validity
  - rule findings relevant to the selection

---

## 32. User workflows enabled by these artefacts

Praxis must support these workflows without custom coding:

1. Build a catalogue of services

- filter by lifecycle, owner, domain
- show coverage to capabilities

2. Build a service × capability matrix

- identify gaps
- drill into missing mapping

3. Build a capability map

- overlay dependency risk and change initiatives
- explore hotspots

4. Build a journey map

- connect pains/gains to services/features and capabilities
- identify change candidates

5. Build an application landscape

- logical to physical mapping
- integration edges
- vendor/platform overlay

6. Build a roadmap (plateaus + release trains)

- compare baseline and target plateau
- show impacted capabilities and systems

---

## 33. What is required next in the Praxis spec (to be complete)

To make this implementable, the next sections should specify:

- the **artefact definition JSON schemas** (formal, versioned)
- the **domain registry keys** (canonical list and stable IDs)
- the **default artefact library** shipped with Praxis (starter set)
- the **UI contract** for rendering layouts (layered, hierarchy, swimlane, matrix)
- the **promotion workflow** for scenario → baseline (how to apply changes safely)
- the **integrity scoring formula** and thresholds used as analytics gates

## 34. Artefact definition schemas (formal, versioned)

All user-facing artefacts (views, catalogues, matrices, maps, templates, reports) must use **explicit, versioned JSON schemas**. This is critical for longevity, migration, and cross-client compatibility.

### 34.1 Common artefact envelope

Every artefact definition shares a common envelope:

```json
{
  "schema_version": "1.0",
  "artefact_kind": "view | catalogue | matrix | map | template | report",
  "id": "stable-uuid",
  "name": "Human readable name",
  "description": "Purpose and usage",
  "owner": "actor-id",
  "visibility": "private | team | partition | public",
  "tags": ["capability", "risk", "cx"],
  "parameters": {
    /* parameter schema */
  },
  "definition": {
    /* artefact-specific */
  }
}
```

`schema_version` is mandatory. Praxis must support forward-compatible readers and explicit migrations.

---

## 35. View definition schema (formal)

### 35.1 View definition body

```json
{
  "scope": {
    "include_master_types": ["Value", "Capability", "Execution", "Technology"],
    "include_domain_types": ["BusinessService", "BusinessCapability"],
    "exclude_domain_types": []
  },
  "seeds": {
    "mode": "explicit | catalogue | none",
    "entity_ids": [],
    "catalogue_ref": null
  },
  "traversal": {
    "allowed_verbs": ["Realises", "IsRealisedBy", "Enables", "Implements"],
    "direction": "both | outbound | inbound",
    "max_depth": 4,
    "stop_at_master_types": ["Technology"]
  },
  "filters": {
    "properties": [{ "field": "status", "op": "Eq", "value": "Active" }],
    "integrity_gates": {
      "min_score": 0.7,
      "on_fail": "warn | block"
    }
  },
  "projection": {
    "node_fields": ["name", "owner", "criticality"],
    "edge_fields": [],
    "aggregations": [{ "kind": "count", "target": "nodes", "group_by": "domain_type" }]
  },
  "visual": {
    "layout": "layered",
    "layer_order": ["Intent", "Value", "Capability", "Execution", "Technology"],
    "group_by": "domain_type",
    "colour_by": "criticality"
  }
}
```

### 35.2 Guarantees

- No traversal without explicit bounds.
- All verbs map to master semantics.
- Views are safe to run at scale.

---

## 36. Catalogue definition schema

### 36.1 Catalogue definition body

```json
{
  "target_types": {
    "master": ["Capability"],
    "domain": ["BusinessCapability"]
  },
  "filters": [{ "field": "lifecycle", "op": "In", "value": ["Active", "Planned"] }],
  "columns": [
    { "field": "name", "label": "Capability" },
    { "field": "owner", "label": "Owner" },
    { "field": "criticality", "label": "Criticality" },
    { "computed": "dependency_score", "label": "Risk" }
  ],
  "grouping": {
    "group_by": ["domain", "level"],
    "show_totals": true
  },
  "sorting": [{ "field": "criticality", "dir": "desc" }],
  "coverage_metrics": [
    {
      "name": "Service Coverage",
      "definition": {
        "related_master_type": "Value",
        "via_verb": "Realises"
      }
    }
  ]
}
```

### 36.2 Behaviour

- Filters must use indexed fields only.
- Computed columns must reference known analytics outputs.
- Coverage metrics are first-class, not ad-hoc.

---

## 37. Matrix definition schema

### 37.1 Matrix definition body

```json
{
  "axis_x": {
    "catalogue_ref": "capability_catalogue_id"
  },
  "axis_y": {
    "catalogue_ref": "logical_app_catalogue_id"
  },
  "relationship": {
    "verbs": ["IsEnabledBy"],
    "direction": "x_to_y"
  },
  "aggregation": {
    "kind": "count | boolean | weighted",
    "weight_source": null
  },
  "cell_drilldown": {
    "include_edges": true,
    "include_paths": false,
    "max_items": 50
  },
  "metrics": {
    "row_coverage": true,
    "column_coverage": true,
    "density": true
  }
}
```

### 37.2 Matrix invariants

- Axes must be catalogue-backed.
- Relationship verbs must be explicit.
- Matrix execution must use projection edges, never raw traversal per cell.

---

## 38. Map definition schemas

### 38.1 Capability map schema

```json
{
  "map_kind": "capability_map",
  "capability_levels": ["L1", "L2", "L3"],
  "group_by": "domain",
  "overlays": [
    { "kind": "criticality", "palette": "heat" },
    { "kind": "dependency_risk", "palette": "red-amber-green" }
  ],
  "labels": {
    "primary": "name",
    "secondary": "owner"
  }
}
```

### 38.2 Journey map schema

```json
{
  "map_kind": "journey_map",
  "journey_type": "CustomerJourney",
  "stage_field": "stage",
  "touchpoint_field": "touchpoint",
  "lanes": ["JourneyStage"],
  "overlays": ["pain", "gain", "metric"]
}
```

### 38.3 Application landscape schema

```json
{
  "map_kind": "application_landscape",
  "logical_grouping": "domain",
  "physical_grouping": "platform",
  "show_integrations": true,
  "integration_types": ["API", "Event", "Batch"],
  "overlays": ["vendor", "lifecycle", "risk"]
}
```

---

## 39. Reports (composed, narrative artefacts)

Reports are **compositions of other artefacts plus commentary**.

### 39.1 Report definition schema

```json
{
  "sections": [
    {
      "title": "Executive Summary",
      "content": { "kind": "text", "template": "…" }
    },
    {
      "title": "Critical Capabilities",
      "content": {
        "kind": "artefact_ref",
        "artefact_id": "capability_rank_view"
      }
    },
    {
      "title": "Risk Hotspots",
      "content": {
        "kind": "matrix_ref",
        "artefact_id": "capability_app_matrix"
      }
    }
  ],
  "parameters": {
    "at_valid_time": { "type": "date", "default": "now" },
    "scenario_id": { "type": "scenario", "default": null }
  }
}
```

Reports must be renderable to:

- interactive UI
- PDF/slide export (future)

---

## 40. Artefact lifecycle and governance

### 40.1 Artefact states

- Draft
- Published
- Deprecated
- Archived

State transitions are time-valid and auditable.

### 40.2 Impact of changes

Changing an artefact:

- invalidates caches
- does not alter underlying data
- does not break historical runs (time-valid definitions)

---

## 41. UI contract (rendering expectations)

Praxis defines **semantic outputs**, not pixels.

### 41.1 UI responsibilities

- render layouts specified by artefact
- respect grouping, ordering, colouring hints
- provide interaction hooks:
  - click → drill-down
  - hover → summary
  - select → context actions

### 41.2 Required UI primitives

Any compliant UI must support:

- hierarchical grouping
- layered graphs
- swimlanes
- matrices (sparse grids)
- overlays and legends
- time slider and scenario selector

---

## 42. Drill-down and reverse navigation

### 42.1 Universal drill-down contract

Given:

- artefact_id
- context (time/scenario)
- selection (node, edge, cell)

Praxis must return:

- underlying entities and relationships
- validity intervals
- integrity findings
- suggested next actions

### 42.2 Reverse navigation

Users must be able to:

- go from matrix cell → view → element editor
- go from ranking → dependency view
- go from map → catalogue filtered to selection

Praxis must generate navigation hints in results.

---

## 43. Promotion workflows (scenario → baseline)

### 43.1 Promotion model

Promotion is **selective replay**:

- user selects changes in scenario
- Praxis identifies corresponding ops
- applies them to baseline with new asserted_at
- preserves original valid times

### 43.2 Conflict handling

If baseline has diverged:

- show domain-level conflicts (e.g. “Service owner changed”)
- allow:
  - accept scenario
  - keep baseline
  - create new plateau

Promotion is a task, not a merge.

---

## 44. Multi-user collaboration and conflict surfacing

### 44.1 Collaboration model

- Mneme handles conflict resolution
- Praxis surfaces conflicts in domain language

Examples:

- “Two owners set for Capability X in July”
- “Overlapping future plans for Application Y”

### 44.2 User resolution tasks

- resolve overlap
- accept both (MV)
- defer decision

Praxis writes resolution ops explicitly.

---

## 45. Integrity scoring model (explicit)

### 45.1 Integrity dimensions

1. Directionality compliance
2. Logical/physical separation
3. Spine completeness
4. Orphan rate
5. Overlap/conflict density

### 45.2 Score calculation

Each dimension yields 0–1 score.
Overall integrity = weighted average (configurable).

Integrity score is:

- attached to artefact results
- stored historically
- used as analytics gate

---

## 46. Extensibility and plugins

Praxis must support plugins that:

- add domain packages
- add artefact templates
- add analytics algorithms
- add integrity rules

Plugin contract:

- declarative registration
- no direct DB access
- versioned compatibility

---

## 47. Testing and validation (artefact-centric)

Required test classes:

- artefact schema validation
- artefact execution determinism
- cross-time consistency
- cross-scenario comparison correctness
- performance bounds (max nodes/edges returned)

---

## 48. What this enables (explicitly)

With this design, a user can:

- Model **anything from customer experience to infrastructure**
- Work entirely through **tasks, views, matrices, maps**
- Travel **backward and forward in time**
- Run **what-if scenarios**
- See **objective rankings and risks**
- Trust analytics because structure is enforced
- Extend the metamodel without breaking tasks
- Share and reuse modelling artefacts safely

At this point, Praxis is a **true enterprise digital twin environment**, not a diagramming tool and not a generic graph database UI.

---

## 49. Formal artefact JSON schemas (machine-readable)

To ensure artefacts are portable, migratable, and safe to execute, Praxis must ship **formal JSON Schema definitions** (draft-2020-12 or later) for every artefact kind.

### 49.1 Schema registry

Praxis includes an internal registry:

- `praxis.schema.view@1.0`
- `praxis.schema.catalogue@1.0`
- `praxis.schema.matrix@1.0`
- `praxis.schema.map.capability@1.0`
- `praxis.schema.map.journey@1.0`
- `praxis.schema.report@1.0`
- `praxis.schema.template@1.0`

Each schema:

- defines required and optional fields
- enforces bounds (e.g. max depth, max verbs)
- disallows arbitrary traversal or unbounded filters
- is versioned independently

### 49.2 Validation points

Schemas are validated:

- on artefact creation
- on artefact update
- on artefact execution (cached artefacts revalidated if schema version changes)

Invalid artefacts:

- cannot be executed
- surface clear validation errors to users/admins

---

## 50. Default artefact library (starter pack)

Praxis must ship with a **default artefact library** so users get value immediately and learn by example.

### 50.1 Required default views

1. **Strategy → Execution Spine**
2. **Service Dependency View**
3. **Capability Dependency View**
4. **Application Support View**
5. **Change Impact View**

Each:

- is read-only by default
- can be copied and customised
- enforces integrity gates

### 50.2 Required default catalogues

- Service Catalogue
- Capability Catalogue
- Application Catalogue (Logical + Physical)
- Project / Programme Catalogue
- Data Asset Catalogue

### 50.3 Required default matrices

- Service × Capability
- Capability × Logical Application
- Logical Application × Physical Application
- Capability × Change Initiative

### 50.4 Required default maps

- Enterprise Capability Map
- Customer Journey Map
- Application Landscape Map
- Technology Platform Map
- Target vs Baseline Plateau Map

These defaults encode **best-practice modelling expectations** and anchor user behaviour.

---

## 51. Performance envelopes and hard limits

Praxis must explicitly define **execution limits** to prevent abuse and guarantee responsiveness.

### 51.1 View limits (configurable, with defaults)

- max nodes returned: 5,000
- max edges returned: 10,000
- max traversal depth: 5
- max execution time: 2s (soft), 5s (hard)

### 51.2 Matrix limits

- max rows: 1,000
- max columns: 1,000
- sparse storage only
- drill-down capped per cell (default 50 items)

### 51.3 Catalogue limits

- pagination mandatory
- max page size: 200
- only indexed filters allowed

### 51.4 Analytics limits

- PageRank max nodes: configurable (default 100k)
- PageRank must run on projection edges only
- global analytics require integrity threshold

Praxis must fail fast with **explainable errors** when limits are exceeded.

---

## 52. Security and tenancy for artefacts

### 52.1 Artefact ownership and ACL

Each artefact includes:

- owner actor
- visibility level
- optional ACL groups

Praxis enforces:

- artefact read permissions
- artefact edit permissions
- artefact execution permissions (important for analytics)

### 52.2 Parameter sanitisation

User-supplied parameters (filters, seeds):

- validated against artefact schema
- cannot inject new traversal verbs or depth
- cannot bypass integrity gates

### 52.3 Tenant isolation

Artefacts are partition-scoped by default.
Cross-partition artefacts require explicit admin configuration.

---

## 53. Offline / embedded mode UX considerations

Praxis must work in **embedded/offline mode** (desktop app, local SQLite).

### 53.1 Behavioural constraints

- no background global analytics unless explicitly triggered
- job execution must be resumable after restart
- artefact execution must degrade gracefully on large data

### 53.2 UX expectations

- artefact execution progress indicators
- warnings when analytics results may be stale
- clear distinction between baseline and scenario when offline

### 53.3 Sync reconciliation

When reconnecting:

- Mneme sync handles ops
- Praxis reruns integrity scans and invalidates artefact caches
- analytics results marked “recomputed after sync”

---

## 54. Artefact migration and compatibility

### 54.1 Forward compatibility

Praxis guarantees:

- newer versions can read older artefact schemas
- deprecated fields are ignored safely
- migrations are explicit and logged

### 54.2 Artefact migration jobs

Artefact schema upgrades are handled via:

- background migration jobs
- dry-run validation before commit
- rollback by restoring previous version

Artefact migrations never mutate historical runs.

---

## 55. Observability and diagnostics (Praxis-level)

Praxis must expose diagnostics for:

- artefact execution time
- cache hit/miss rates
- integrity score trends
- analytics run counts and durations
- error frequency by artefact kind

These diagnostics are:

- queryable via API
- optionally visualised via default reports

---

## 56. Explainability as a first-class requirement

### 56.1 Explain artefact execution

For any artefact run, Praxis can explain:

- why nodes/edges were included or excluded
- which filters applied
- which integrity gates passed/failed

### 56.2 Explain analytics

For rankings and scores:

- top inbound contributors
- top dependency paths
- change deltas between scenarios or times

Explainability outputs are structured and UI-friendly.

---

## 57. Failure modes and user-visible behaviour

### 57.1 Incomplete model

If the model is incomplete:

- artefacts still run
- results are annotated with confidence warnings
- integrity gaps are highlighted as actionable items

### 57.2 Conflicting data

If conflicts exist:

- MV values surfaced explicitly
- matrices and views reflect ambiguity
- users are guided to resolution tasks

---

## 58. API stability and evolution

### 58.1 API guarantees

- task APIs remain stable across metamodel expansion
- new domain types never break existing tasks
- artefact schemas evolve independently

### 58.2 Deprecation policy

- minimum one major version grace period
- deprecation warnings surfaced in artefact validation
- automated migration suggestions where possible

---

## 59. Minimal implementation checklist (Praxis v1)

To be considered complete, Praxis v1 must include:

- master type system + domain registry
- metamodel package publishing
- task-oriented authoring APIs
- views, catalogues, matrices, and maps
- artefact storage and execution
- integrity scoring and gating
- analytics orchestration (PageRank)
- explainability APIs
- offline-safe execution
- default artefact library

---

## 60. Final position

This design makes Praxis:

- **Task-oriented for users**
- **Conceptually rich for architects**
- **Structurally disciplined for analytics**
- **Time-aware and scenario-driven**
- **Extensible without losing coherence**

The combination of:

- master structural anchors
- domain extensibility
- artefact-based interaction
- strict separation from storage

is what gives Praxis its **digital twin power** rather than becoming “yet another EA repository”.

If you want to continue further, the remaining optional deep dives would be:

- formal **integrity rule DSL**
- **change promotion semantics** at scale
- **multi-partition federation**
- **streaming analytics and live dashboards**
- **AI-assisted modelling suggestions (guarded by integrity rules)**

## 61. UX system specification (Praxis artefacts → user experience)

This section specifies **what the UX must support**, not UI aesthetics. It defines the **interaction contract** between Praxis artefacts and any consuming UI (desktop, web, embedded).

The UX layer is a _projection_ of Praxis artefacts. It must not invent semantics.

---

## 62. Page model: headers, footers, layout, and composition

Praxis artefacts render into a **Page Model**. This allows:

- consistent presentation
- printable/exportable outputs
- composition of multiple artefacts
- dashboarding without special cases

### 62.1 Page definition

A page is a layout container that references artefacts.

```json
{
  "page_id": "stable-uuid",
  "name": "Capability Risk Overview",
  "layout": "single | two-column | grid | freeform",
  "page_size": "A4 | A3 | 16:9 | custom",
  "orientation": "portrait | landscape",
  "header": {
    /* header spec */
  },
  "footer": {
    /* footer spec */
  },
  "sections": [
    /* ordered sections */
  ]
}
```

Pages are artefacts (`artefact_kind = "page"`).

---

### 62.2 Header specification

Headers are declarative and dynamic.

```json
{
  "left": { "kind": "text", "value": "Enterprise Architecture Overview" },
  "center": { "kind": "artefact_param", "param": "scenario_name" },
  "right": { "kind": "time_context", "format": "as at {date}" }
}
```

Header elements may reference:

- static text
- artefact parameters
- time/scenario context
- organisation branding tokens (logo, colour)

---

### 62.3 Footer specification

Footers support auditability and export.

```json
{
  "left": { "kind": "page_number", "format": "Page {n} of {total}" },
  "center": { "kind": "text", "value": "Generated by Aideon Praxis" },
  "right": { "kind": "timestamp", "format": "Generated {datetime}" }
}
```

---

### 62.4 Page sections

Each section embeds an artefact with layout hints.

```json
{
  "section_id": "s1",
  "title": "Top Risk Capabilities",
  "artefact_ref": "capability_rank_view",
  "height": "auto | fixed",
  "interaction": "interactive | static",
  "export_mode": "full | summary"
}
```

This enables:

- dashboards
- executive reports
- printable packs
- embedded views

---

## 63. Diagram model (generic, reusable)

All visualisations are instances of a **single diagram model**. Different diagrams are configurations of this model.

### 63.1 Core diagram primitives

Every diagram is composed of:

- **Visual nodes**
- **Visual edges**
- **Containers / groups**
- **Overlays**
- **Legends**

Praxis returns a _diagram specification_, not pixels.

---

### 63.2 Diagram specification schema (core)

```json
{
  "diagram_kind": "graph | hierarchy | swimlane | matrix | map",
  "nodes": [
    /* visual node specs */
  ],
  "edges": [
    /* visual edge specs */
  ],
  "containers": [
    /* groupings */
  ],
  "overlays": [
    /* heatmaps, badges */
  ],
  "interactions": {
    /* interaction rules */
  },
  "layout": {
    /* layout hints */
  }
}
```

---

## 64. Visual node specification

```json
{
  "id": "entity-id",
  "label": "Payments Capability",
  "icon": "capability",
  "shape": "rectangle | rounded | hexagon",
  "size": { "w": 120, "h": 60 },
  "style": {
    "fill": "by_overlay",
    "border": "default"
  },
  "badges": [
    { "kind": "count", "value": 12 },
    { "kind": "warning", "severity": "high" }
  ],
  "tooltips": {
    "summary": "Critical customer-facing capability",
    "details": ["Owner: Ops", "Lifecycle: Active"]
  }
}
```

Nodes must be:

- stable by ID
- re-renderable when data changes
- selectable and drillable

---

## 65. Visual edge specification

```json
{
  "id": "edge-id",
  "src": "entity-id",
  "dst": "entity-id",
  "verb": "Realises",
  "style": {
    "line": "solid | dashed",
    "arrow": "forward",
    "thickness": "by_weight"
  },
  "weight": 0.72,
  "tooltips": {
    "summary": "Service realises capability"
  }
}
```

Edges:

- reflect master semantics
- may be visually weighted (dependency strength, frequency, count)

---

## 66. Containers and grouping

Containers represent:

- capability domains
- journey stages
- logical platforms
- organisational units

```json
{
  "id": "container-id",
  "label": "Customer Capabilities",
  "node_ids": ["n1", "n2"],
  "layout": "stacked | grid | free",
  "style": {
    "border": "dotted",
    "background": "light"
  }
}
```

Containers enable:

- capability maps
- swimlanes
- nested landscapes

---

## 67. Heat maps and rich overlays (explicit)

Overlays are **data-driven visual layers**.

### 67.1 Overlay definition

```json
{
  "overlay_id": "dependency_risk",
  "applies_to": "node | edge | container",
  "source": "analytics | property | integrity",
  "field": "dependency_score",
  "scale": {
    "type": "continuous",
    "min": 0,
    "max": 1,
    "palette": "green-yellow-red"
  },
  "legend": {
    "title": "Dependency Risk",
    "units": "relative"
  }
}
```

### 67.2 Supported overlay types

- continuous heatmaps (risk, criticality, cost)
- discrete categories (lifecycle, domain)
- badges (counts, alerts)
- sparklines (trend over time)

Multiple overlays can be active simultaneously.

---

## 68. Interactive diagram models (dynamic visuals)

### 68.1 Parameter-driven interactivity

Diagrams may bind to parameters:

- time
- scenario
- filters (domain, owner)
- thresholds (risk cutoff)

When a parameter changes:

- Praxis re-runs artefact with new context
- diagram updates incrementally

### 68.2 Example: interactive threshold

“Highlight only dependencies with weight > X”

This is:

- a parameter on the artefact
- bound to an overlay filter
- UI slider adjusts X
- diagram re-renders edges live

---

## 69. Time-aware visualisation

### 69.1 Time slider integration

All diagrams must support:

- time slider
- milestone snapping (plateaus, releases)
- plan vs actual toggles

Praxis provides:

- time bounds
- change markers (events that alter the diagram)
- delta annotations

### 69.2 Change visualisation

When time changes:

- nodes/edges appear/disappear
- overlays fade or intensify
- change badges appear:
  - “added”
  - “removed”
  - “changed”

This is critical for roadmaps and target architecture views.

---

## 70. Matrix UX specifics

### 70.1 Matrix rendering contract

- sparse grid rendering
- sticky headers
- row/column totals always visible
- heatmap overlays per cell

### 70.2 Cell interactions

Clicking a cell opens:

- list of connecting relationships
- mini-view showing paths
- “create missing link” task (if gap)

---

## 71. Catalogue UX specifics

### 71.1 Catalogue interactions

- column sorting
- faceted filtering
- inline indicators (coverage, warnings)
- bulk selection for actions

### 71.2 Inline analytics

Catalogues may show:

- dependency score
- change impact score
- integrity warnings

Users can pivot directly from catalogue to:

- view
- matrix
- map

---

## 72. Custom diagrams (user-defined layouts)

### 72.1 Diagram composition mode

Advanced users can:

- start from a view
- manually arrange nodes
- save layout as a **custom diagram artefact**

The underlying model remains unchanged.

### 72.2 Stored diagram layout

```json
{
  "diagram_kind": "custom",
  "node_positions": {
    "n1": { "x": 120, "y": 40 },
    "n2": { "x": 300, "y": 40 }
  },
  "locked": true
}
```

Layouts are time-invariant unless explicitly versioned.

---

## 73. UX affordances for integrity and guidance

### 73.1 Visual integrity cues

- dashed edges for weak/inferred links
- red outlines for violations
- greyed nodes for incomplete modelling

### 73.2 Suggested actions

Praxis returns **suggested tasks** with artefacts:

- “Link service to at least one capability”
- “Add logical application for this physical app”
- “Resolve overlapping plans”

These are surfaced contextually.

---

## 74. Undo, redo, and interaction safety

### 74.1 UX expectations

- all user actions are reversible (logically)
- undo = write compensating op
- redo = reapply op

Praxis must surface:

- action history
- effective dates affected
- scenario context

---

## 75. Export and sharing UX

### 75.1 Export formats

- PDF (page-based artefacts)
- PNG/SVG (diagrams)
- CSV (catalogues, matrices)
- JSON (artefact definitions)

### 75.2 Shareable artefacts

Artefacts can be:

- shared by link
- embedded in other systems
- exported/imported as files

Artefact exports are **definition-only**, not data.

---

## 76. Accessibility and scale considerations

### 76.1 Accessibility

- colour-blind safe palettes
- keyboard navigation
- screen-reader friendly catalogue views

### 76.2 Scale handling

- progressive rendering
- level-of-detail reduction
- clustering for large diagrams
- explicit “expand” actions

---

## 77. UX non-goals (explicit)

Praxis UX will not:

- allow arbitrary free-form drawing detached from the model
- allow editing underlying graph structure visually without task mediation
- allow unbounded pan/zoom on millions of elements

---

## 78. Summary: what the UX contract achieves

This UX specification ensures:

- **Consistency**: every artefact behaves predictably
- **Power**: interactive, time-aware, analytics-rich visuals
- **Safety**: integrity and semantics preserved
- **Composability**: pages, reports, dashboards
- **Extensibility**: new artefacts and overlays without UX redesign

Together with Mneme, Praxis now defines:

- the **meaning layer**
- the **interaction layer**
- and the **visual contract**

for a genuinely usable, extensible enterprise digital twin.

## 79. Collaborative UX patterns (review, commentary, governance)

Praxis must support **collaborative modelling without turning the model into a chat log**. Collaboration is layered, scoped, and auditable.

### 79.1 Commenting and annotations

Annotations are **non-structural artefacts** attached to:

- elements
- relationships
- artefacts (views, matrices, maps)
- time slices (e.g. “as at Q2”)

They never alter analytics inputs.

#### Annotation model

- target (entity/edge/artefact)
- scope (partition / scenario / time window)
- body (markdown-lite)
- severity (note / concern / decision)
- author + asserted time

Annotations are:

- filterable
- exportable
- optionally included in reports

---

### 79.2 Review and approval flows

Praxis supports **lightweight governance**, not heavyweight workflow engines.

Supported patterns:

- “Request review” on:
  - metamodel package changes
  - scenario promotion
  - artefact publication

- reviewers can:
  - comment
  - approve
  - request changes

Approval outcomes are stored as **decisions**, not locks.

---

### 79.3 Decision records (ADR-style)

Praxis supports structured decisions:

- decision statement
- context artefact references
- chosen option
- alternatives considered
- date and actors

Decisions link to:

- change initiatives
- plateaus
- artefacts

This enables architectural memory without polluting the model.

---

## 80. AI-assisted UX (guarded, explainable)

AI is **assistive**, not authoritative. It must never bypass rules.

### 80.1 AI suggestion categories

- Suggested missing links (“Most services of this type realise a capability like X”)
- Suggested classifications (domain, lifecycle)
- Anomaly detection (“This capability has unusually high dependency density”)
- Artefact recommendations (“Users often create a matrix like…”)

### 80.2 Guardrails

AI suggestions:

- are never auto-applied
- are surfaced as _proposals_
- include explanation (“based on similar patterns”)
- respect integrity rules

### 80.3 Training boundaries

AI operates on:

- anonymised structural patterns
- public or partition-local data only
- no cross-tenant leakage

---

## 81. Federation and multi-partition UX

Praxis supports **federated views** across partitions while preserving isolation.

### 81.1 Federation use cases

- group-level capability map
- shared platform landscapes
- supplier ecosystem modelling

### 81.2 Federation rules

- read-only by default
- explicit mapping between partitions
- no cross-partition writes unless configured

### 81.3 UX representation

Federated elements:

- visually marked
- scoped labels (“External”, “Upstream”)
- limited drill-down

---

## 82. Notifications and change awareness

Praxis must support **signal without noise**.

### 82.1 Notification triggers

- integrity threshold crossed
- high-impact change introduced
- scenario promotion completed
- analytics ranking shift beyond threshold

### 82.2 Delivery modes

- in-app
- digest (daily/weekly)
- artefact-specific subscriptions

Notifications reference artefacts, not raw data.

---

## 83. Formal UI conformance and golden tests

To ensure UI consistency across implementations:

### 83.1 Golden artefacts

Praxis ships:

- canonical artefact definitions
- canonical execution outputs
- expected diagram specs

UI implementations must:

- render these artefacts identically (semantically)
- pass automated snapshot tests

### 83.2 Interaction contract tests

- drill-down correctness
- parameter binding correctness
- undo/redo behaviour
- time slider behaviour

---

## 84. Accessibility and internationalisation

### 84.1 Accessibility requirements

- keyboard navigation for all artefacts
- screen-reader friendly catalogues
- colour-independent overlays
- zoom without loss of context

### 84.2 Internationalisation

- artefact labels localisable
- date/time formats locale-aware
- right-to-left layout support (where applicable)

---

## 85. Performance transparency to users

Praxis must expose **why something is slow**, not just that it is.

### 85.1 UX indicators

- “This view is large (4,200 nodes)”
- “Analytics last updated 2 hours ago”
- “Integrity warnings may affect results”

### 85.2 User controls

- scope narrowing suggestions
- switch to summary mode
- request background execution

---

## 86. Failure handling UX

### 86.1 Partial results

If an artefact exceeds limits:

- return partial results
- mark as incomplete
- explain what was omitted

### 86.2 Degraded mode

If analytics unavailable:

- show last known result
- mark stale
- provide refresh option

---

## 87. Audit, traceability, and compliance UX

### 87.1 Audit views

Users can:

- inspect change history of an element
- see who changed what, when, and why
- replay state at a past date

### 87.2 Compliance artefacts

Praxis supports:

- evidence views (controls → systems → data)
- traceability matrices
- exportable audit packs

---

## 88. Education and guided adoption

Praxis must teach users how to model well.

### 88.1 Guided modelling

- inline hints (“Most services link to 1–3 capabilities”)
- warnings framed as advice
- example artefacts provided

### 88.2 Progressive disclosure

- basic users see simple tasks
- advanced users unlock:
  - custom artefacts
  - scenario promotion
  - analytics tuning

---

## 89. Extending Praxis safely

### 89.1 Extension points

- new domain packages
- new artefact kinds (rare)
- new analytics modules
- new overlays

### 89.2 Compatibility rules

Extensions must:

- declare master type mappings
- declare integrity implications
- pass schema validation
- not weaken existing rules silently

---

## 90. End-to-end lifecycle summary

Praxis supports the full lifecycle:

1. **Model**
   Task-oriented authoring across CX → capability → technology → change

2. **Explore**
   Views, catalogues, matrices, maps

3. **Analyse**
   Rankings, risk, impact, diagnostics

4. **Plan**
   Scenarios, plateaus, roadmaps

5. **Decide**
   Reviews, decisions, approvals

6. **Communicate**
   Pages, reports, exports

7. **Evolve**
   Metamodel extensions, artefact refinement

All while remaining:

- time-aware
- scenario-aware
- analytically sound
- storage-agnostic

---

## 91. Final completeness check

With sections 1–91, the Praxis specification now explicitly covers:

- conceptual and structural foundations
- metamodel extensibility without loss of coherence
- task-oriented interaction model
- artefact system (views, matrices, maps, reports, pages)
- UX contracts (layout, interaction, visual semantics)
- time, scenario, and analytics integration
- collaboration, governance, and audit
- performance, safety, and scalability
- extensibility and future-proofing

At this point, Praxis is fully specified as:

> **A task-oriented, analytically disciplined, time-aware enterprise digital twin environment.**

## 92. Concrete default metamodel (canonical IDs, fields, verbs)

This section removes ambiguity by defining **what ships out of the box**. It does not limit extensibility; it anchors it.

---

### 92.1 Master type registry (canonical, immutable)

Each master type has a **stable ID**, never regenerated.

| Master Type | ID (symbolic)   | Purpose                          |
| ----------- | --------------- | -------------------------------- |
| Actor       | `MT_ACTOR`      | Parties, roles, organisations    |
| Intent      | `MT_INTENT`     | Motivation, goals, drivers       |
| Value       | `MT_VALUE`      | Services, products, journeys     |
| Capability  | `MT_CAPABILITY` | Stable enterprise abilities      |
| Execution   | `MT_EXECUTION`  | Processes, controls, information |
| Technology  | `MT_TECHNOLOGY` | Logical/physical tech landscape  |
| Structure   | `MT_STRUCTURE`  | Containers, org, locations       |
| Change      | `MT_CHANGE`     | Projects, plateaus, delivery     |

All domain types must declare exactly one `master_type_id`.

---

### 92.2 Master relationship semantics (canonical verbs)

Each semantic verb has:

- stable ID
- direction
- dependency meaning (for analytics)
- allowed master-type pairs

| Verb         | ID                  | Direction | Meaning                          |
| ------------ | ------------------- | --------- | -------------------------------- |
| Influences   | `RV_INFLUENCES`     | src → dst | src impacts dst                  |
| Realises     | `RV_REALISES`       | src → dst | value realised by capability     |
| IsRealisedBy | `RV_IS_REALISED_BY` | src → dst | capability realised by execution |
| Enables      | `RV_ENABLES`        | src → dst | src enables dst                  |
| Implements   | `RV_IMPLEMENTS`     | src → dst | logical implemented by physical  |
| BelongsTo    | `RV_BELONGS_TO`     | src → dst | containment                      |
| Changes      | `RV_CHANGES`        | src → dst | change affects target            |
| DependsOn    | `RV_DEPENDS_ON`     | src → dst | generic dependency               |
| Consumes     | `RV_CONSUMES`       | src → dst | data/information consumption     |
| Produces     | `RV_PRODUCES`       | src → dst | data/information production      |

Analytics assumes **incoming edges = dependency** unless overridden.

---

### 92.3 Required default domain types (subset, extensible)

Each domain type declares:

- domain type ID
- parent domain type (optional)
- master type
- default fields
- allowed verbs

#### 92.3.1 Customer & experience

- Customer → Actor
- Persona → Actor
- Journey → Value
- JourneyStage → Value
- Touchpoint → Value
- Pain → Intent
- Gain → Intent

#### 92.3.2 Strategy & motivation

- Vision → Intent
- Goal → Intent
- Objective → Intent
- Driver → Intent
- BrandAttribute → Intent

#### 92.3.3 Services & products

- Product → Value
- BusinessService → Value
- Feature → Value
- ValueStream → Value

#### 92.3.4 Capability & execution

- BusinessCapability → Capability
- Process → Execution
- OperatingModel → Execution
- Control → Execution
- InformationAsset → Execution

#### 92.3.5 Data & integration

- ConceptualDataModel → Execution
- DataEntity → Execution
- API → Execution
- BatchInterface → Execution
- Event → Execution

#### 92.3.6 Technology

- LogicalApplication → Technology
- ApplicationModule → Technology
- PhysicalApplication → Technology
- TechnologyPlatform → Technology
- PhysicalTechnology → Technology

#### 92.3.7 Organisation & structure

- Role → Actor
- OrgUnit → Structure
- Location → Structure
- Vendor → Actor
- Contract → Structure

#### 92.3.8 Change & delivery

- Project → Change
- Programme → Change
- Portfolio → Change
- Plateau → Change
- ReleaseTrain → Change
- Milestone → Change

---

### 92.4 Default field set (cross-cutting)

These fields exist on _all_ domain types unless overridden:

| Field       | Type              | Notes                         |
| ----------- | ----------------- | ----------------------------- |
| name        | string            | required                      |
| description | text              | optional                      |
| owner       | reference (Actor) | optional                      |
| lifecycle   | enum              | Planned / Active / Retired    |
| criticality | int (1–5)         | analytics-friendly            |
| tags        | set<string>       | OR-set                        |
| status      | enum              | Draft / Approved / Deprecated |

---

## 93. Default artefact definitions (shipped examples)

Praxis must ship **ready-to-run artefacts**.

---

### 93.1 View: Strategy → Execution Spine

- Scope: Intent, Value, Capability, Execution, Technology
- Verbs: Influences, Realises, IsRealisedBy, Enables, Implements
- Depth: 5
- Layout: layered
- Integrity gate: ≥0.75

Purpose: reveal how strategy connects to delivery.

---

### 93.2 Matrix: Capability × Logical Application

- Rows: BusinessCapability catalogue
- Columns: LogicalApplication catalogue
- Verb: Enables (Capability ← LogicalApp)
- Aggregation: boolean
- Coverage metrics: row + column

Purpose: reveal coverage gaps and hotspots.

---

### 93.3 Map: Enterprise Capability Map

- Hierarchy: Capability L1/L2/L3
- Overlays: criticality, dependency risk
- Grouping: domain

Purpose: executive-level capability overview.

---

### 93.4 Catalogue: Application Catalogue

- Target: LogicalApplication + PhysicalApplication
- Columns: name, owner, lifecycle, vendor
- Filters: lifecycle != Retired

Purpose: inventory and governance.

---

## 94. End-to-end reference flows (build confidence)

### 94.1 Customer journey to capability impact

1. Create Customer → Persona → Journey → Touchpoints
2. Link Touchpoints → Services (Consumes)
3. Services → Capabilities (Realises)
4. Capabilities → Apps (Enables)
5. Run Capability Risk View
6. Heatmap reveals customer-critical capability gaps

---

### 94.2 Target architecture planning

1. Create Plateau “Target 2027”
2. In scenario:
   - add new Logical Applications
   - retire legacy Physical Applications

3. Link changes via `Changes`
4. Compare baseline vs target
5. Run impact analysis
6. Promote selected changes

---

## 95. Implementation sequencing (risk-aware)

This is not a plan, but a **dependency-aware build order**.

1. Master types + master verbs (non-negotiable)
2. Domain registry + default domain types
3. Task-oriented authoring APIs
4. Time/scenario correctness
5. Views + catalogues
6. Matrices
7. Maps + diagram model
8. Analytics (PageRank)
9. Integrity scoring + gating
10. UX interactivity (time slider, overlays)
11. Collaboration + governance
12. AI assist (optional)

---

## 96. Known risk hotspots (explicit)

- Allowing arbitrary domain verbs → breaks analytics
- Allowing multi-inheritance → breaks defaults & explainability
- Skipping logical/physical separation → invalid PageRank
- Unbounded traversal → performance collapse
- Free-form diagram editing → semantic drift

These must be guarded from day one.

---

## 97. Long-term extensibility (designed-in)

The following can be added without breaking anything:

- New domain packages
- New analytics algorithms
- New overlays and heatmaps
- New artefact kinds (rare)
- Federated partitions
- Streaming dashboards

Because:

- master anchors are stable
- artefacts are declarative
- tasks target roles, not types

---

## 98. Final architectural assertion

With this specification, Praxis is:

- **Not** a diagramming tool
- **Not** a generic graph editor
- **Not** a static EA repository

It is a **time-aware, scenario-capable, analytically disciplined enterprise digital twin**.

The combination of:

- master structural anchors
- rich domain extensibility
- task-oriented interaction
- artefact-based exploration
- explainable analytics

is what allows users to model _anything_ without losing meaning.

---

## 99. Ready-for-build checklist

A team can now build Praxis if they can answer “yes” to:

- Do master types and verbs exist as code constants?
- Are domain types versioned and package-based?
- Do tasks avoid exposing graph mechanics?
- Are artefacts declarative and schema-validated?
- Are views, matrices, maps bounded and explainable?
- Is time/scenario applied everywhere?
- Are analytics gated by integrity?
- Can UX render diagrams from specs alone?

If yes, this spec is complete.

---

## 100. Close

This completes the **Praxis design specification** to a level suitable for:

- multi-year evolution
- multiple UI implementations
- embedded and server deployments
- serious analytical use (including PageRank)

## 101. Canonical executable artefacts (ready-to-run)

This section provides **concrete, executable artefact definitions** that can be loaded directly into Praxis and rendered by any compliant UI. These are intended as **golden references** for implementation and testing.

---

## 101.1 Canonical View: Strategy → Execution Spine

### Artefact metadata

```json
{
  "schema_version": "1.0",
  "artefact_kind": "view",
  "id": "VIEW_STRATEGY_EXECUTION_SPINE",
  "name": "Strategy to Execution Spine",
  "description": "End-to-end trace from intent through value, capability, execution, and technology.",
  "visibility": "partition",
  "tags": ["strategy", "capability", "execution"]
}
```

### Definition

```json
{
  "scope": {
    "include_master_types": ["Intent", "Value", "Capability", "Execution", "Technology"]
  },
  "seeds": { "mode": "none" },
  "traversal": {
    "allowed_verbs": ["Influences", "Realises", "IsRealisedBy", "Enables", "Implements"],
    "direction": "outbound",
    "max_depth": 5,
    "stop_at_master_types": ["Technology"]
  },
  "filters": {
    "integrity_gates": {
      "min_score": 0.75,
      "on_fail": "warn"
    }
  },
  "projection": {
    "node_fields": ["name", "owner", "criticality", "lifecycle"]
  },
  "visual": {
    "layout": "layered",
    "layer_order": ["Intent", "Value", "Capability", "Execution", "Technology"],
    "group_by": "master_type"
  }
}
```

---

## 101.2 Canonical Catalogue: Business Capability Catalogue

```json
{
  "schema_version": "1.0",
  "artefact_kind": "catalogue",
  "id": "CATALOGUE_BUSINESS_CAPABILITY",
  "name": "Business Capability Catalogue",
  "description": "All business capabilities with ownership and risk indicators.",
  "visibility": "partition",
  "definition": {
    "target_types": {
      "master": ["Capability"],
      "domain": ["BusinessCapability"]
    },
    "columns": [
      { "field": "name", "label": "Capability" },
      { "field": "owner", "label": "Owner" },
      { "field": "criticality", "label": "Criticality" },
      { "computed": "dependency_score", "label": "Dependency Risk" }
    ],
    "sorting": [{ "field": "criticality", "dir": "desc" }]
  }
}
```

---

## 101.3 Canonical Matrix: Capability × Logical Application

```json
{
  "schema_version": "1.0",
  "artefact_kind": "matrix",
  "id": "MATRIX_CAPABILITY_LOGICAL_APP",
  "name": "Capability to Logical Application Coverage",
  "visibility": "partition",
  "definition": {
    "axis_x": {
      "catalogue_ref": "CATALOGUE_BUSINESS_CAPABILITY"
    },
    "axis_y": {
      "catalogue_ref": "CATALOGUE_LOGICAL_APPLICATION"
    },
    "relationship": {
      "verbs": ["Enables"],
      "direction": "y_to_x"
    },
    "aggregation": { "kind": "boolean" },
    "metrics": {
      "row_coverage": true,
      "column_coverage": true,
      "density": true
    }
  }
}
```

---

## 101.4 Canonical Map: Enterprise Capability Map

```json
{
  "schema_version": "1.0",
  "artefact_kind": "map",
  "id": "MAP_ENTERPRISE_CAPABILITY",
  "name": "Enterprise Capability Map",
  "visibility": "partition",
  "definition": {
    "map_kind": "capability_map",
    "capability_levels": ["L1", "L2", "L3"],
    "group_by": "domain",
    "overlays": [
      { "kind": "criticality", "palette": "heat" },
      { "kind": "dependency_risk", "palette": "red-amber-green" }
    ]
  }
}
```

---

## 102. Reference UI rendering contract (minimum viable)

This defines the **absolute minimum UI behaviour** required to render Praxis artefacts correctly.

### 102.1 Required UI capabilities

A compliant UI must support:

- Time slider (valid time)
- Scenario selector
- Artefact parameter binding
- Diagram rendering from spec
- Matrix rendering (sparse)
- Catalogue rendering with pagination
- Drill-down interactions
- Overlay toggles
- Export (PNG, CSV, PDF)

---

## 103. Reference diagram rendering flow

1. UI receives `ViewResult` or `DiagramSpec`
2. UI:
   - instantiates visual nodes
   - instantiates edges
   - applies containers
   - applies overlays

3. User interacts:
   - selects node → drill-down request
   - changes parameter → re-run artefact

4. UI never:
   - infers relationships
   - alters semantics
   - caches authoritative state

---

## 104. Interactive example: changing a variable

### Use case

User adjusts **risk threshold** slider.

### Behaviour

- Slider bound to artefact parameter `min_dependency_score`
- Praxis re-executes view
- Diagram:
  - fades nodes below threshold
  - highlights edges contributing to high scores

- Integrity score recalculated and displayed

This interaction is **purely declarative**.

---

## 105. Page assembly example (print-ready)

```json
{
  "page_id": "PAGE_EXEC_CAPABILITY_RISK",
  "page_size": "A3",
  "orientation": "landscape",
  "header": {
    "left": { "kind": "text", "value": "Enterprise Architecture Risk Overview" },
    "right": { "kind": "time_context" }
  },
  "sections": [
    {
      "title": "Critical Capabilities",
      "artefact_ref": "CATALOGUE_BUSINESS_CAPABILITY",
      "export_mode": "summary"
    },
    {
      "title": "Capability Risk Map",
      "artefact_ref": "MAP_ENTERPRISE_CAPABILITY",
      "export_mode": "full"
    }
  ]
}
```

---

## 106. Golden test dataset (minimum)

To validate correctness, ship a **golden dataset**:

- 3 strategies
- 5 services
- 8 capabilities (L1–L3)
- 6 logical applications
- 3 physical applications
- 2 platforms
- 1 programme with 2 projects
- 1 target plateau

This dataset must:

- produce stable PageRank ordering
- expose at least one integrity violation
- demonstrate scenario delta

---

## 107. Golden test assertions

Automated tests must assert:

- View outputs identical across DB backends
- Matrix coverage % stable
- PageRank ordering stable within tolerance
- Integrity score stable
- Diagram spec JSON identical

---

## 108. Developer ergonomics (important)

Praxis implementation must provide:

- strongly typed artefact builders (Rust)
- artefact schema validation errors with clear messages
- human-readable logs for artefact execution
- ability to dump artefact execution plans (debug)

---

## 109. What “done” looks like

Praxis is “done” when:

- a new user can model CX → capability → app → change
- an exec can see risk hotspots in one view
- planners can compare target vs baseline
- analytics results are explainable
- UI teams can build without reinterpreting semantics

---

## 110. Final handoff

At this point you have:

- a complete **Mneme storage engine spec**
- a complete **Praxis meaning, interaction, artefact, and UX spec**
- executable artefact definitions
- UI contracts
- test datasets and assertions

This is sufficient to:

- split work across teams
- build iteratively without rework
- evolve the metamodel safely
- support real enterprise-scale analysis

## Rust: core traits and types for artefact execution

```rust
// praxis_core/src/types.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type PartitionId = String;
pub type ScenarioId = String;
pub type ActorId = String;
pub type EntityId = String;
pub type EdgeId = String;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Context {
    pub partition_id: PartitionId,
    pub scenario_id: Option<ScenarioId>,
    pub actor_id: ActorId,

    /// Valid time slice (domain time). ISO-8601 date or datetime.
    pub at_valid_time: String,

    /// Optional audit mode (what did we know then?). ISO-8601 datetime or HLC string.
    pub as_of_asserted_at: Option<String>,

    /// Plan vs Actual etc.
    pub layer: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParameterValue(pub serde_json::Value);

pub type Parameters = HashMap<String, ParameterValue>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ArtefactEnvelope {
    pub schema_version: String,
    pub artefact_kind: ArtefactKind,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: Option<String>,
    pub tags: Option<Vec<String>>,
    pub parameters: Option<serde_json::Value>, // parameter schema
    pub definition: serde_json::Value,         // artefact-specific body
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtefactKind {
    View,
    Catalogue,
    Matrix,
    Map,
    Template,
    Report,
    Page,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionMeta {
    pub artefact_id: String,
    pub artefact_kind: ArtefactKind,
    pub schema_version: String,
    pub ctx: Context,
    pub params_hash: String,
    pub definition_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Finding {
    pub severity: String, // note|warn|error
    pub rule_id: Option<String>,
    pub subject_id: Option<String>,
    pub message: String,
    pub details: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub node_count: usize,
    pub edge_count: usize,
    pub integrity_score: Option<f64>,
    pub warnings: Vec<Finding>,
    pub execution_ms: u64,
}

/// Common UX output: diagram spec (renderable by UI)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiagramSpec {
    pub diagram_kind: String, // graph|hierarchy|swimlane|matrix|map|custom
    pub nodes: Vec<VisualNode>,
    pub edges: Vec<VisualEdge>,
    pub containers: Vec<VisualContainer>,
    pub overlays: Vec<OverlaySpec>,
    pub layout: serde_json::Value,
    pub interactions: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VisualNode {
    pub id: EntityId,
    pub label: String,
    pub kind: String,
    pub icon: Option<String>,
    pub badges: Vec<serde_json::Value>,
    pub tooltips: Option<serde_json::Value>,
    pub style: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VisualEdge {
    pub id: EdgeId,
    pub src: EntityId,
    pub dst: EntityId,
    pub verb: String,
    pub weight: Option<f64>,
    pub tooltips: Option<serde_json::Value>,
    pub style: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VisualContainer {
    pub id: String,
    pub label: String,
    pub node_ids: Vec<EntityId>,
    pub layout: Option<String>,
    pub style: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OverlaySpec {
    pub overlay_id: String,
    pub applies_to: String, // node|edge|container
    pub source: String,     // analytics|property|integrity
    pub field: String,
    pub scale: serde_json::Value,
    pub legend: serde_json::Value,
}

/// Outputs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewResult {
    pub meta: ExecutionMeta,
    pub summary: ExecutionSummary,
    pub nodes: Vec<NodeResult>,
    pub edges: Vec<EdgeResult>,
    pub groups: Vec<serde_json::Value>,
    pub layouts: serde_json::Value,
    pub diagram: DiagramSpec,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeResult {
    pub id: EntityId,
    pub kind: String,
    pub label: String,
    pub fields: serde_json::Value,
    pub groups: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgeResult {
    pub id: EdgeId,
    pub verb: String,
    pub src: EntityId,
    pub dst: EntityId,
    pub weight: Option<f64>,
    pub fields: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatalogueResult {
    pub meta: ExecutionMeta,
    pub summary: ExecutionSummary,
    pub columns: Vec<ColumnSpec>,
    pub rows: Vec<RowResult>,
    pub groups: Vec<GroupResult>,
    pub page: PageInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColumnSpec {
    pub key: String,
    pub label: String,
    pub kind: String, // field|computed
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RowResult {
    pub id: EntityId,
    pub values: HashMap<String, serde_json::Value>,
    pub warnings: Vec<Finding>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupResult {
    pub key: String,
    pub title: String,
    pub row_ids: Vec<EntityId>,
    pub totals: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PageInfo {
    pub cursor: Option<String>,
    pub has_more: bool,
    pub page_size: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MatrixResult {
    pub meta: ExecutionMeta,
    pub summary: ExecutionSummary,
    pub rows: Vec<AxisItem>,
    pub cols: Vec<AxisItem>,
    pub cells: Vec<Cell>,
    pub row_totals: Vec<AxisTotal>,
    pub col_totals: Vec<AxisTotal>,
    pub diagram: DiagramSpec, // matrix rendered as diagram too
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AxisItem {
    pub id: EntityId,
    pub label: String,
    pub fields: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cell {
    pub r: EntityId,
    pub c: EntityId,
    pub v: serde_json::Value,
    pub edge_ids: Vec<EdgeId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AxisTotal {
    pub id: EntityId,
    pub total: serde_json::Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapResult {
    pub meta: ExecutionMeta,
    pub summary: ExecutionSummary,
    pub diagram: DiagramSpec,
}
```

```rust
// praxis_core/src/traits.rs
use crate::types::*;
use async_trait::async_trait;

#[derive(thiserror::Error, Debug)]
pub enum PraxisError {
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Execution limit exceeded: {0}")]
    Limits(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

#[async_trait]
pub trait ArtefactStore: Send + Sync {
    async fn get_artefact(&self, ctx: &Context, artefact_id: &str) -> Result<ArtefactEnvelope, PraxisError>;
    async fn put_artefact(&self, ctx: &Context, artefact: ArtefactEnvelope) -> Result<(), PraxisError>;
    async fn list_artefacts(&self, ctx: &Context, kind: Option<ArtefactKind>, limit: usize, cursor: Option<String>)
        -> Result<(Vec<ArtefactEnvelope>, Option<String>), PraxisError>;
}

#[async_trait]
pub trait SchemaRegistry: Send + Sync {
    async fn validate(&self, artefact: &ArtefactEnvelope) -> Result<(), PraxisError>;
    async fn migrate_if_needed(&self, artefact: ArtefactEnvelope) -> Result<ArtefactEnvelope, PraxisError>;
}

#[async_trait]
pub trait IntegrityService: Send + Sync {
    async fn get_integrity_score(&self, ctx: &Context) -> Result<f64, PraxisError>;
    async fn get_findings_for_selection(&self, ctx: &Context, ids: &[String]) -> Result<Vec<Finding>, PraxisError>;
}

/// Mneme facade used by Praxis (keep it narrow).
#[async_trait]
pub trait MnemeFacade: Send + Sync {
    async fn traverse_view(
        &self,
        ctx: &Context,
        verbs: &[String],
        direction: &str,
        seeds: &[EntityId],
        max_depth: u32,
        node_limit: usize,
        edge_limit: usize,
    ) -> Result<(Vec<NodeResult>, Vec<EdgeResult>), PraxisError>;

    async fn read_fields_batch(
        &self,
        ctx: &Context,
        entity_ids: &[EntityId],
        fields: &[String], // Praxis field keys; facade resolves mapping
    ) -> Result<std::collections::HashMap<EntityId, serde_json::Value>, PraxisError>;

    async fn list_entities(
        &self,
        ctx: &Context,
        kind_filters: &[String],      // domain kinds
        indexed_filters: serde_json::Value,
        limit: usize,
        cursor: Option<String>,
    ) -> Result<(Vec<EntityId>, Option<String>), PraxisError>;

    async fn get_projection_edges(
        &self,
        ctx: &Context,
        verb_filters: &[String],
        endpoint_filter: Option<&[EntityId]>,
        limit: usize,
        cursor: Option<String>,
    ) -> Result<(Vec<EdgeResult>, Option<String>), PraxisError>;
}

#[async_trait]
pub trait ArtefactExecutor: Send + Sync {
    async fn execute_view(&self, ctx: &Context, artefact: &ArtefactEnvelope, params: &Parameters) -> Result<ViewResult, PraxisError>;
    async fn execute_catalogue(&self, ctx: &Context, artefact: &ArtefactEnvelope, params: &Parameters, cursor: Option<String>)
        -> Result<CatalogueResult, PraxisError>;
    async fn execute_matrix(&self, ctx: &Context, artefact: &ArtefactEnvelope, params: &Parameters) -> Result<MatrixResult, PraxisError>;
    async fn execute_map(&self, ctx: &Context, artefact: &ArtefactEnvelope, params: &Parameters) -> Result<MapResult, PraxisError>;
}

/// High-level API: fetch + validate + migrate + execute.
#[async_trait]
pub trait PraxisArtefactApi: Send + Sync {
    async fn run(&self, ctx: &Context, artefact_id: &str, params: Parameters, cursor: Option<String>)
        -> Result<serde_json::Value, PraxisError>;

    async fn drilldown(&self, ctx: &Context, selection: DrilldownSelection)
        -> Result<serde_json::Value, PraxisError>;
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DrilldownSelection {
    pub artefact_id: Option<String>,
    pub selection_kind: String, // node|edge|cell|container
    pub ids: Vec<String>,
    pub context: Option<serde_json::Value>,
}
```

---

## TypeScript SDK stubs (task + artefact execution)

```ts
// praxis-ts/src/types.ts
export type PartitionId = string;
export type ScenarioId = string;
export type ActorId = string;

export interface Context {
  partitionId: PartitionId;
  scenarioId?: ScenarioId;
  actorId: ActorId;
  atValidTime: string; // ISO8601
  asOfAssertedAt?: string; // optional audit time/HLC
  layer?: 'Plan' | 'Actual' | string;
}

export type ArtefactKind = 'view' | 'catalogue' | 'matrix' | 'map' | 'template' | 'report' | 'page';

export interface ArtefactEnvelope {
  schemaVersion: string;
  artefactKind: ArtefactKind;
  id: string;
  name: string;
  description?: string;
  visibility?: string;
  tags?: string[];
  parameters?: any; // param schema
  definition: any; // kind-specific
}

export interface ExecutionSummary {
  nodeCount: number;
  edgeCount: number;
  integrityScore?: number;
  warnings: Array<{
    severity: string;
    message: string;
    ruleId?: string;
    subjectId?: string;
    details?: any;
  }>;
  executionMs: number;
}

export interface DiagramSpec {
  diagramKind: string;
  nodes: any[];
  edges: any[];
  containers: any[];
  overlays: any[];
  layout: any;
  interactions: any;
}

export interface ViewResult {
  summary: ExecutionSummary;
  nodes: any[];
  edges: any[];
  groups?: any[];
  layouts?: any;
  diagram: DiagramSpec;
}

export interface CatalogueResult {
  summary: ExecutionSummary;
  columns: Array<{ key: string; label: string; kind: string }>;
  rows: Array<{ id: string; values: Record<string, any>; warnings?: any[] }>;
  groups?: any[];
  page: { cursor?: string; hasMore: boolean; pageSize: number };
}

export interface MatrixResult {
  summary: ExecutionSummary;
  rows: any[];
  cols: any[];
  cells: any[];
  rowTotals: any[];
  colTotals: any[];
  diagram: DiagramSpec;
}

export interface MapResult {
  summary: ExecutionSummary;
  diagram: DiagramSpec;
}
```

```ts
// praxis-ts/src/client.ts
import {
  ArtefactEnvelope,
  CatalogueResult,
  Context,
  MapResult,
  MatrixResult,
  ViewResult,
} from './types';

export interface PraxisClientOptions {
  baseUrl: string;
  fetchImpl?: typeof fetch;
}

export class PraxisClient {
  private baseUrl: string;
  private fetchImpl: typeof fetch;

  constructor(opts: PraxisClientOptions) {
    this.baseUrl = opts.baseUrl.replace(/\/+$/, '');
    this.fetchImpl = opts.fetchImpl ?? fetch;
  }

  async getArtefact(ctx: Context, artefactId: string): Promise<ArtefactEnvelope> {
    const res = await this.fetchImpl(
      `${this.baseUrl}/artefacts/${encodeURIComponent(artefactId)}`,
      {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ ctx }),
      },
    );
    if (!res.ok) throw new Error(`getArtefact failed: ${res.status}`);
    return res.json();
  }

  async runView(
    ctx: Context,
    artefactId: string,
    params: Record<string, any> = {},
  ): Promise<ViewResult> {
    return this.run(ctx, artefactId, params) as Promise<ViewResult>;
  }

  async runCatalogue(
    ctx: Context,
    artefactId: string,
    params: Record<string, any> = {},
    cursor?: string,
  ): Promise<CatalogueResult> {
    return this.run(ctx, artefactId, params, cursor) as Promise<CatalogueResult>;
  }

  async runMatrix(
    ctx: Context,
    artefactId: string,
    params: Record<string, any> = {},
  ): Promise<MatrixResult> {
    return this.run(ctx, artefactId, params) as Promise<MatrixResult>;
  }

  async runMap(
    ctx: Context,
    artefactId: string,
    params: Record<string, any> = {},
  ): Promise<MapResult> {
    return this.run(ctx, artefactId, params) as Promise<MapResult>;
  }

  async run(
    ctx: Context,
    artefactId: string,
    params: Record<string, any> = {},
    cursor?: string,
  ): Promise<any> {
    const res = await this.fetchImpl(`${this.baseUrl}/run/${encodeURIComponent(artefactId)}`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ ctx, params, cursor }),
    });
    if (!res.ok) throw new Error(`run failed: ${res.status}`);
    return res.json();
  }

  async drilldown(
    ctx: Context,
    selection: { artefactId?: string; selectionKind: string; ids: string[]; context?: any },
  ): Promise<any> {
    const res = await this.fetchImpl(`${this.baseUrl}/drilldown`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ ctx, selection }),
    });
    if (!res.ok) throw new Error(`drilldown failed: ${res.status}`);
    return res.json();
  }
}
```

---

## Reference UI rendering skeleton (React + minimal diagram + overlays)

This is intentionally barebones: it renders `DiagramSpec` in SVG, supports overlay toggles, and re-runs on time/scenario changes.

```tsx
// ui/src/App.tsx
import React, { useMemo, useState } from 'react';
import { PraxisClient } from './praxisClient';
import { DiagramView } from './DiagramView';
import { Context } from './types';

const client = new PraxisClient({ baseUrl: 'http://localhost:8080' });

export default function App() {
  const [partitionId, setPartitionId] = useState('P1');
  const [scenarioId, setScenarioId] = useState<string | undefined>(undefined);
  const [atValidTime, setAtValidTime] = useState('2026-01-02');
  const [artefactId, setArtefactId] = useState('VIEW_STRATEGY_EXECUTION_SPINE');
  const [overlay, setOverlay] = useState('dependency_risk');
  const [result, setResult] = useState<any>(null);

  const ctx: Context = useMemo(
    () => ({
      partitionId,
      scenarioId,
      actorId: 'A1',
      atValidTime,
      layer: 'Actual',
    }),
    [partitionId, scenarioId, atValidTime],
  );

  async function run() {
    const out = await client.run(ctx, artefactId, { overlay });
    setResult(out);
  }

  return (
    <div style={{ display: 'grid', gridTemplateColumns: '320px 1fr', height: '100vh' }}>
      <div style={{ padding: 12, borderRight: '1px solid #ddd' }}>
        <h3>Praxis Artefacts</h3>

        <div>
          <label>Partition</label>
          <br />
          <input value={partitionId} onChange={(e) => setPartitionId(e.target.value)} />
        </div>

        <div>
          <label>Scenario (optional)</label>
          <br />
          <input
            value={scenarioId ?? ''}
            onChange={(e) => setScenarioId(e.target.value || undefined)}
          />
        </div>

        <div>
          <label>Valid time</label>
          <br />
          <input value={atValidTime} onChange={(e) => setAtValidTime(e.target.value)} />
        </div>

        <div>
          <label>Artefact ID</label>
          <br />
          <input value={artefactId} onChange={(e) => setArtefactId(e.target.value)} />
        </div>

        <div>
          <label>Overlay</label>
          <br />
          <input value={overlay} onChange={(e) => setOverlay(e.target.value)} />
        </div>

        <button onClick={run} style={{ marginTop: 10 }}>
          Run
        </button>

        {result?.summary && (
          <div style={{ marginTop: 12 }}>
            <div>Nodes: {result.summary.nodeCount}</div>
            <div>Edges: {result.summary.edgeCount}</div>
            <div>Integrity: {result.summary.integrityScore ?? 'n/a'}</div>
            <div>Exec ms: {result.summary.executionMs}</div>
          </div>
        )}
      </div>

      <div style={{ padding: 12 }}>
        {result?.diagram ? (
          <DiagramView diagram={result.diagram} onSelect={(sel) => console.log('select', sel)} />
        ) : (
          <div>Run an artefact to render.</div>
        )}
      </div>
    </div>
  );
}
```

```tsx
// ui/src/DiagramView.tsx
import React, { useMemo } from 'react';

type DiagramSpec = {
  diagramKind: string;
  nodes: Array<{ id: string; label: string; style?: any; tooltips?: any }>;
  edges: Array<{ id: string; src: string; dst: string; style?: any; verb?: string }>;
  overlays: Array<any>;
  layout: any;
};

export function DiagramView({
  diagram,
  onSelect,
}: {
  diagram: DiagramSpec;
  onSelect: (sel: any) => void;
}) {
  // Minimal deterministic layout: arrange nodes in a grid (placeholder for real layout engines).
  const positions = useMemo(() => {
    const cols = Math.max(1, Math.ceil(Math.sqrt(diagram.nodes.length)));
    const spacingX = 180;
    const spacingY = 110;
    const pos: Record<string, { x: number; y: number }> = {};
    diagram.nodes.forEach((n, idx) => {
      const x = (idx % cols) * spacingX + 80;
      const y = Math.floor(idx / cols) * spacingY + 60;
      pos[n.id] = { x, y };
    });
    return pos;
  }, [diagram.nodes]);

  return (
    <svg width="100%" height="100%" viewBox="0 0 1400 900" style={{ border: '1px solid #eee' }}>
      {/* edges */}
      {diagram.edges.map((e) => {
        const a = positions[e.src];
        const b = positions[e.dst];
        if (!a || !b) return null;
        return (
          <g
            key={e.id}
            onClick={() => onSelect({ kind: 'edge', id: e.id })}
            style={{ cursor: 'pointer' }}
          >
            <line x1={a.x} y1={a.y} x2={b.x} y2={b.y} strokeWidth={2} />
            <text x={(a.x + b.x) / 2} y={(a.y + b.y) / 2} fontSize={10}>
              {e.verb ?? ''}
            </text>
          </g>
        );
      })}

      {/* nodes */}
      {diagram.nodes.map((n) => {
        const p = positions[n.id];
        return (
          <g
            key={n.id}
            onClick={() => onSelect({ kind: 'node', id: n.id })}
            style={{ cursor: 'pointer' }}
          >
            <rect x={p.x - 60} y={p.y - 24} width={120} height={48} rx={10} />
            <text x={p.x} y={p.y + 4} textAnchor="middle" fontSize={12}>
              {n.label}
            </text>
          </g>
        );
      })}
    </svg>
  );
}
```

---

## Golden dataset: JSON (entities + links + time + scenario)

This is a minimal dataset consistent with the default spine and useful for PageRank and integrity checks.

### 1) Entities (nodes)

```json
{
  "entities": [
    { "id": "S_GOAL_1", "kind": "Goal", "name": "Improve customer experience", "master": "Intent" },
    { "id": "S_OBJ_1", "kind": "Objective", "name": "Reduce onboarding time", "master": "Intent" },
    { "id": "V_JOURNEY_1", "kind": "Journey", "name": "Open an account", "master": "Value" },
    {
      "id": "V_SERVICE_1",
      "kind": "BusinessService",
      "name": "Digital onboarding",
      "master": "Value"
    },
    {
      "id": "V_SERVICE_2",
      "kind": "BusinessService",
      "name": "Identity verification",
      "master": "Value"
    },
    {
      "id": "C_CAP_L1",
      "kind": "BusinessCapability",
      "name": "Customer Management",
      "master": "Capability",
      "level": "L1",
      "domain": "Customer"
    },
    {
      "id": "C_CAP_L2_1",
      "kind": "BusinessCapability",
      "name": "Onboarding",
      "master": "Capability",
      "level": "L2",
      "domain": "Customer"
    },
    {
      "id": "C_CAP_L2_2",
      "kind": "BusinessCapability",
      "name": "KYC",
      "master": "Capability",
      "level": "L2",
      "domain": "Customer"
    },
    { "id": "E_PROCESS_1", "kind": "Process", "name": "Verify identity", "master": "Execution" },
    {
      "id": "T_LOG_APP_1",
      "kind": "LogicalApplication",
      "name": "Onboarding Workflow",
      "master": "Technology"
    },
    {
      "id": "T_LOG_APP_2",
      "kind": "LogicalApplication",
      "name": "KYC Rules Engine",
      "master": "Technology"
    },
    {
      "id": "T_PHY_APP_1",
      "kind": "PhysicalApplication",
      "name": "Core Banking Suite",
      "master": "Technology"
    },
    {
      "id": "T_PLATFORM_1",
      "kind": "TechnologyPlatform",
      "name": "Integration Platform",
      "master": "Technology"
    },
    { "id": "CH_PROG_1", "kind": "Programme", "name": "Digital CX uplift", "master": "Change" },
    { "id": "CH_PLATEAU_BASE", "kind": "Plateau", "name": "Baseline 2026", "master": "Change" },
    { "id": "CH_PLATEAU_TGT", "kind": "Plateau", "name": "Target 2027", "master": "Change" }
  ]
}
```

### 2) Relationships (edges)

```json
{
  "edges": [
    { "id": "E1", "verb": "Influences", "src": "S_GOAL_1", "dst": "S_OBJ_1" },
    { "id": "E2", "verb": "Influences", "src": "S_OBJ_1", "dst": "V_SERVICE_1" },

    { "id": "E3", "verb": "BelongsTo", "src": "V_JOURNEY_1", "dst": "V_SERVICE_1" },

    { "id": "E4", "verb": "Realises", "src": "V_SERVICE_1", "dst": "C_CAP_L2_1" },
    { "id": "E5", "verb": "Realises", "src": "V_SERVICE_2", "dst": "C_CAP_L2_2" },

    { "id": "E6", "verb": "BelongsTo", "src": "C_CAP_L2_1", "dst": "C_CAP_L1" },
    { "id": "E7", "verb": "BelongsTo", "src": "C_CAP_L2_2", "dst": "C_CAP_L1" },

    { "id": "E8", "verb": "IsRealisedBy", "src": "C_CAP_L2_2", "dst": "E_PROCESS_1" },

    { "id": "E9", "verb": "Enables", "src": "T_LOG_APP_1", "dst": "C_CAP_L2_1" },
    { "id": "E10", "verb": "Enables", "src": "T_LOG_APP_2", "dst": "C_CAP_L2_2" },

    { "id": "E11", "verb": "Implements", "src": "T_LOG_APP_1", "dst": "T_PHY_APP_1" },
    { "id": "E12", "verb": "Implements", "src": "T_LOG_APP_2", "dst": "T_PLATFORM_1" },

    { "id": "E13", "verb": "Changes", "src": "CH_PROG_1", "dst": "V_SERVICE_1" },
    { "id": "E14", "verb": "BelongsTo", "src": "CH_PLATEAU_TGT", "dst": "CH_PROG_1" }
  ]
}
```

### 3) Scenario delta (what-if)

```json
{
  "scenario": {
    "scenario_id": "SCN_TARGET_2027",
    "changes": [
      {
        "op": "add_entity",
        "id": "T_LOG_APP_3",
        "kind": "LogicalApplication",
        "name": "Digital ID Service",
        "master": "Technology"
      },
      {
        "op": "add_edge",
        "id": "E15",
        "verb": "Enables",
        "src": "T_LOG_APP_3",
        "dst": "C_CAP_L2_2"
      },
      {
        "op": "add_edge",
        "id": "E16",
        "verb": "Implements",
        "src": "T_LOG_APP_3",
        "dst": "T_PLATFORM_1"
      }
    ]
  }
}
```

---

## Import script skeleton (Praxis → Mneme via task APIs)

This script assumes Praxis exposes:

- `createElement(kind, id, name, attrs, ctx)`
- `link(verb, edgeId, srcId, dstId, ctx)`

```ts
// tools/importGolden.ts
import { PraxisClient } from '../praxis-ts/src/client';

const client = new PraxisClient({ baseUrl: 'http://localhost:8080' });

async function run() {
  const ctx = {
    partitionId: 'P1',
    actorId: 'A1',
    atValidTime: '2026-01-02',
    layer: 'Actual',
  };

  const nodes = (await import('./golden_entities.json')).default.entities as any[];
  const edges = (await import('./golden_edges.json')).default.edges as any[];

  for (const n of nodes) {
    await client.run(ctx as any, 'TASK_CREATE_ELEMENT', {
      kind: n.kind,
      id: n.id,
      name: n.name,
      attributes: { master: n.master, level: n.level, domain: n.domain },
    });
  }

  for (const e of edges) {
    await client.run(ctx as any, 'TASK_LINK', {
      verb: e.verb,
      edgeId: e.id,
      srcId: e.src,
      dstId: e.dst,
    });
  }

  // Scenario changes
  const scn = (await import('./golden_scenario.json')).default.scenario;
  const scnCtx = { ...ctx, scenarioId: scn.scenario_id };

  for (const ch of scn.changes) {
    if (ch.op === 'add_entity') {
      await client.run(scnCtx as any, 'TASK_CREATE_ELEMENT', {
        kind: ch.kind,
        id: ch.id,
        name: ch.name,
        attributes: { master: ch.master },
      });
    } else if (ch.op === 'add_edge') {
      await client.run(scnCtx as any, 'TASK_LINK', {
        verb: ch.verb,
        edgeId: ch.id,
        srcId: ch.src,
        dstId: ch.dst,
      });
    }
  }

  console.log('Golden dataset imported.');
}

run().catch((e) => {
  console.error(e);
  process.exit(1);
});
```

---

## Minimal task IDs expected by the import script

If you prefer not to use a generic `run(TASK_…)` endpoint, expose explicit endpoints instead. If you do keep it generic, these task IDs must exist:

- `TASK_CREATE_ELEMENT`
  - params: `{ kind, id, name, attributes }`

- `TASK_LINK`
  - params: `{ verb, edgeId, srcId, dstId }`

They map to Praxis authoring operations and then to Mneme writes.
