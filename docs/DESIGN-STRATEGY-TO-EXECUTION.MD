# Strategy-to-Execution Design - Aideon Suite

## Purpose

Capture the core strategy-to-execution design for Aideon Suite: the normative map from motivation
through value delivery to technology and change, the functional capabilities required to support
that flow, and the canonical viewpoints architects work with. This document is the suite-level
source of truth.

## 1. Strategy-to-Execution Map (normative)

The strategy-to-execution spine is:

**Intent -> Value -> Capability -> Execution -> Technology -> Change**

Key expectations:

- Every domain type inherits from a master type.
- Domain verbs map to master semantics (directionality is enforced).
- Connectivity gaps along the spine reduce integrity scores and must be reported.

## 2. Functional capabilities

Praxis must support the following functional capabilities:

- Task-oriented authoring (create element, link, set attributes, move/contain).
- Metamodel packages and registry with stable IDs.
- Artefact execution: views, catalogues, matrices, maps, reports.
- Integrity and quality gates (directionality, logical/physical separation, spine completeness).
- Analytics orchestration with explainability.
- Scenario overlays and time-context reads.

## 3. Time, colour, and story modes

Time is expressed via:

- Valid time (what is true at time T)
- Asserted time (what we knew at time A)
- Layer (Plan vs Actual)

UX should surface these explicitly and allow time scrubbing and scenario comparison.

## 4. Viewpoint library (ISO 42010-aligned)

Canonical viewpoints derived from the strategy-to-execution model:

- Strategy & Motivation
- Capability Map
- Value Stream
- Service Blueprint
- Information
- Application Portfolio
- Integration
- Cloud/Deployment
- Roadmap
- Solution Architecture

These viewpoints are implemented as artefacts (views, catalogues, matrices, maps, reports) and
should be stored and executed through Praxis.
