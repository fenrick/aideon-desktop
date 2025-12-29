use std::collections::BTreeMap;

use crate::temporal::{
    ChangeSet, DiffPatch, EdgeTombstone, EdgeVersion, NodeTombstone, NodeVersion,
};
use serde::{Deserialize, Serialize};

use crate::error::{PraxisError, PraxisResult};
use crate::meta::MetaModelRegistry;

/// Deterministic key for edges stored inside a graph snapshot.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct EdgeKey {
    id: Option<String>,
    from: String,
    to: String,
}

impl EdgeKey {
    fn new(version: &EdgeVersion) -> Self {
        Self {
            id: version.id.clone(),
            from: version.from.clone(),
            to: version.to.clone(),
        }
    }

    fn matches_tombstone(&self, tombstone: &EdgeTombstone) -> bool {
        self.from == tombstone.from && self.to == tombstone.to
    }
}

/// Aggregated metrics for a snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct SnapshotStats {
    pub node_count: usize,
    pub edge_count: usize,
}

/// Immutable graph snapshot used when materialising commits.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GraphSnapshot {
    nodes: BTreeMap<String, NodeVersion>,
    edges: BTreeMap<EdgeKey, EdgeVersion>,
}

impl GraphSnapshot {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn stats(&self) -> SnapshotStats {
        SnapshotStats {
            node_count: self.nodes.len(),
            edge_count: self.edges.len(),
        }
    }

    pub fn apply(
        &self,
        change: &ChangeSet,
        registry: &MetaModelRegistry,
    ) -> PraxisResult<GraphSnapshot> {
        let mut next = self.clone();
        apply_node_changes(&mut next, change, registry)?;
        apply_edge_changes(&mut next, change, registry)?;
        next.validate()?;
        Ok(next)
    }

    pub fn diff(&self, other: &GraphSnapshot) -> DiffPatch {
        let mut patch = DiffPatch::default();

        for (id, node) in other.nodes.iter() {
            if !self.nodes.contains_key(id) {
                patch.node_adds.push(node.clone());
            } else if self.nodes.get(id) != Some(node) {
                patch.node_mods.push(node.clone());
            }
        }
        for id in self.nodes.keys() {
            if !other.nodes.contains_key(id) {
                patch.node_dels.push(NodeTombstone { id: id.clone() });
            }
        }

        for (key, edge) in other.edges.iter() {
            if !self.edges.contains_key(key) {
                patch.edge_adds.push(edge.clone());
            } else if self.edges.get(key) != Some(edge) {
                patch.edge_mods.push(edge.clone());
            }
        }
        for key in self.edges.keys() {
            if !other.edges.contains_key(key) {
                patch.edge_dels.push(EdgeTombstone {
                    from: key.from.clone(),
                    to: key.to.clone(),
                });
            }
        }

        patch
    }

    fn validate(&self) -> PraxisResult<()> {
        for edge in self.edges.values() {
            if !self.nodes.contains_key(&edge.from) || !self.nodes.contains_key(&edge.to) {
                return Err(PraxisError::IntegrityViolation {
                    message: format!(
                        "edge '{}' references missing endpoint(s)",
                        edge.id
                            .clone()
                            .unwrap_or_else(|| format!("{}->{}", edge.from, edge.to))
                    ),
                });
            }
        }
        Ok(())
    }

    pub(crate) fn node(&self, id: &str) -> Option<&NodeVersion> {
        self.nodes.get(id)
    }

    pub(crate) fn has_node(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
    }

    pub(crate) fn edge(&self, edge: &EdgeVersion) -> Option<&EdgeVersion> {
        self.edges.get(&EdgeKey::new(edge))
    }

    pub(crate) fn has_edge(&self, edge: &EdgeVersion) -> bool {
        self.edge(edge).is_some()
    }

    pub(crate) fn has_edge_tombstone(&self, tombstone: &EdgeTombstone) -> bool {
        self.edges
            .keys()
            .any(|key| key.matches_tombstone(tombstone))
    }
}

fn sanitize_node(node: &NodeVersion) -> NodeVersion {
    let mut copy = NodeVersion {
        id: node.id.clone(),
        r#type: node.r#type.clone(),
        props: node.props.clone(),
    };
    // To-do: validate props against schema once meta-model solidifies.
    if let Some(props) = &copy.props
        && props.is_null()
    {
        copy.props = None;
    }
    copy
}

fn apply_node_changes(
    snapshot: &mut GraphSnapshot,
    change: &ChangeSet,
    registry: &MetaModelRegistry,
) -> PraxisResult<()> {
    // Node deletes first — we will validate edges afterwards to forbid dangling refs.
    for tombstone in &change.node_deletes {
        if snapshot.nodes.remove(&tombstone.id).is_none() {
            return Err(PraxisError::ValidationFailed {
                message: format!("node '{}' does not exist for delete", tombstone.id),
            });
        }
    }

    for node in &change.node_creates {
        registry.validate_node(node)?;
        if snapshot.nodes.contains_key(&node.id) {
            return Err(PraxisError::ValidationFailed {
                message: format!("node '{}' already exists", node.id),
            });
        }
        snapshot.nodes.insert(node.id.clone(), sanitize_node(node));
    }

    // Node updates are replace-by-id for now (TODO: support partial updates with schema merge).
    for node in &change.node_updates {
        registry.validate_node(node)?;
        if !snapshot.nodes.contains_key(&node.id) {
            return Err(PraxisError::ValidationFailed {
                message: format!("node '{}' missing for update", node.id),
            });
        }
        snapshot.nodes.insert(node.id.clone(), sanitize_node(node));
    }
    Ok(())
}

fn apply_edge_changes(
    snapshot: &mut GraphSnapshot,
    change: &ChangeSet,
    registry: &MetaModelRegistry,
) -> PraxisResult<()> {
    // Edge deletes
    for tombstone in &change.edge_deletes {
        remove_edges_matching(&mut snapshot.edges, tombstone)?;
    }

    for edge in &change.edge_creates {
        ensure_endpoints_exist(&snapshot.nodes, edge)?;
        let from_type = node_type(&snapshot.nodes, &edge.from)?;
        let to_type = node_type(&snapshot.nodes, &edge.to)?;
        registry.validate_edge(edge, &from_type, &to_type)?;
        let key = EdgeKey::new(edge);
        if snapshot.edges.contains_key(&key) {
            return Err(PraxisError::ValidationFailed {
                message: format!(
                    "edge '{}' already exists",
                    edge.id
                        .clone()
                        .unwrap_or_else(|| format!("{}->{}", edge.from, edge.to))
                ),
            });
        }
        let rel_type = relationship_type(edge)?;
        if !registry.allows_duplicate(rel_type) {
            assert_no_duplicate_edge(&snapshot.edges, edge, rel_type)?;
        }
        snapshot.edges.insert(key, sanitize_edge(edge));
    }

    // Edge updates — replace existing entry by id when present, otherwise resolve by endpoints.
    for edge in &change.edge_updates {
        ensure_endpoints_exist(&snapshot.nodes, edge)?;
        let key = resolve_edge_key(&snapshot.edges, edge)?;
        snapshot.edges.remove(&key);
        let from_type = node_type(&snapshot.nodes, &edge.from)?;
        let to_type = node_type(&snapshot.nodes, &edge.to)?;
        registry.validate_edge(edge, &from_type, &to_type)?;
        let rel_type = relationship_type(edge)?;
        if !registry.allows_duplicate(rel_type) {
            assert_no_duplicate_edge(&snapshot.edges, edge, rel_type)?;
        }
        snapshot
            .edges
            .insert(EdgeKey::new(edge), sanitize_edge(edge));
    }
    Ok(())
}

fn resolve_edge_key(
    edges: &BTreeMap<EdgeKey, EdgeVersion>,
    edge: &EdgeVersion,
) -> PraxisResult<EdgeKey> {
    if let Some(id) = &edge.id {
        return edges
            .keys()
            .find(|k| k.id.as_deref() == Some(id.as_str()))
            .cloned()
            .ok_or_else(|| PraxisError::ValidationFailed {
                message: format!("edge '{}' missing for update", id),
            });
    }

    // Resolve by endpoints; require a single match to maintain determinism.
    let mut matches: Vec<EdgeKey> = edges
        .keys()
        .filter(|k| k.from == edge.from && k.to == edge.to)
        .cloned()
        .collect();
    if matches.is_empty() {
        return Err(PraxisError::ValidationFailed {
            message: format!("edge '{}->{}' missing for update", edge.from, edge.to),
        });
    }
    if matches.len() > 1 {
        return Err(PraxisError::ValidationFailed {
            message: format!(
                "edge '{}->{}' update is ambiguous ({} matches)",
                edge.from,
                edge.to,
                matches.len()
            ),
        });
    }
    Ok(matches.pop().expect("single match"))
}

fn sanitize_edge(edge: &EdgeVersion) -> EdgeVersion {
    let mut copy = EdgeVersion {
        id: edge.id.clone(),
        from: edge.from.clone(),
        to: edge.to.clone(),
        r#type: edge.r#type.clone(),
        directed: edge.directed,
        props: edge.props.clone(),
    };
    if let Some(props) = &copy.props
        && props.is_null()
    {
        copy.props = None;
    }
    copy
}

fn node_type(nodes: &BTreeMap<String, NodeVersion>, node_id: &str) -> PraxisResult<String> {
    let node = nodes
        .get(node_id)
        .ok_or_else(|| PraxisError::ValidationFailed {
            message: format!("node '{node_id}' missing"),
        })?;
    node.r#type
        .clone()
        .ok_or_else(|| PraxisError::ValidationFailed {
            message: format!("node '{node_id}' missing type"),
        })
}

fn relationship_type(edge: &EdgeVersion) -> PraxisResult<&str> {
    edge.r#type
        .as_deref()
        .ok_or_else(|| PraxisError::ValidationFailed {
            message: format!("edge '{}->{}' missing type", edge.from, edge.to),
        })
}

fn assert_no_duplicate_edge(
    edges: &BTreeMap<EdgeKey, EdgeVersion>,
    candidate: &EdgeVersion,
    rel_type: &str,
) -> PraxisResult<()> {
    let exists = edges.values().any(|edge| {
        edge.from == candidate.from
            && edge.to == candidate.to
            && edge.r#type.as_deref() == Some(rel_type)
    });
    if exists {
        return Err(PraxisError::ValidationFailed {
            message: format!(
                "relationship '{rel_type}' already exists between '{}' and '{}'",
                candidate.from, candidate.to
            ),
        });
    }
    Ok(())
}

fn ensure_endpoints_exist(
    nodes: &BTreeMap<String, NodeVersion>,
    edge: &EdgeVersion,
) -> PraxisResult<()> {
    if !nodes.contains_key(&edge.from) || !nodes.contains_key(&edge.to) {
        return Err(PraxisError::ValidationFailed {
            message: format!(
                "edge '{}' references missing node(s)",
                edge.id
                    .clone()
                    .unwrap_or_else(|| format!("{}->{}", edge.from, edge.to))
            ),
        });
    }
    Ok(())
}

fn remove_edges_matching(
    edges: &mut BTreeMap<EdgeKey, EdgeVersion>,
    tombstone: &EdgeTombstone,
) -> PraxisResult<()> {
    let keys: Vec<EdgeKey> = edges
        .keys()
        .filter(|k| k.matches_tombstone(tombstone))
        .cloned()
        .collect();
    if keys.is_empty() {
        return Err(PraxisError::ValidationFailed {
            message: format!(
                "edge '{}->{}' does not exist for delete",
                tombstone.from, tombstone.to
            ),
        });
    }
    for key in keys {
        edges.remove(&key);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::{
        MetaModelDocument, MetaRelationship, MetaRelationshipValidation, MetaType,
        MetaValidationRules,
    };
    use serde_json::Value;
    use std::collections::BTreeMap as StdBTreeMap;

    fn registry(allow_duplicate: Option<bool>) -> MetaModelRegistry {
        let mut rel_rules = StdBTreeMap::new();
        rel_rules.insert(
            "rel".to_string(),
            MetaRelationshipValidation {
                allow_self: Some(true),
                allow_duplicate,
            },
        );
        let doc = MetaModelDocument {
            version: "v1".into(),
            description: None,
            types: vec![
                MetaType {
                    id: "A".into(),
                    label: None,
                    category: None,
                    extends: None,
                    attributes: vec![],
                    effect_types: None,
                },
                MetaType {
                    id: "B".into(),
                    label: None,
                    category: None,
                    extends: None,
                    attributes: vec![],
                    effect_types: None,
                },
            ],
            relationships: vec![MetaRelationship {
                id: "rel".into(),
                label: None,
                from: vec!["A".into()],
                to: vec!["B".into()],
                directed: Some(true),
                multiplicity: None,
                attributes: vec![],
            }],
            validation: Some(MetaValidationRules {
                attributes: None,
                relationships: Some(rel_rules),
            }),
        };
        MetaModelRegistry::from_document(doc).expect("registry")
    }

    #[test]
    fn sanitize_node_strips_null_props() {
        let node = NodeVersion {
            id: "n1".into(),
            r#type: Some("A".into()),
            props: Some(Value::Null),
        };
        let sanitized = sanitize_node(&node);
        assert!(sanitized.props.is_none());
    }

    #[test]
    fn apply_rejects_edges_with_missing_endpoints() {
        let registry = registry(None);
        let snapshot = GraphSnapshot::empty();
        let change = ChangeSet {
            edge_creates: vec![EdgeVersion {
                id: Some("e1".into()),
                from: "missing".into(),
                to: "missing2".into(),
                r#type: Some("rel".into()),
                directed: None,
                props: None,
            }],
            ..ChangeSet::default()
        };
        let err = snapshot.apply(&change, &registry).unwrap_err();
        assert!(matches!(err, PraxisError::ValidationFailed { .. }));
        assert!(err.to_string().contains("missing node"));
    }

    #[test]
    fn apply_rejects_deleting_missing_node() {
        let registry = registry(None);
        let snapshot = GraphSnapshot::empty();
        let change = ChangeSet {
            node_deletes: vec![NodeTombstone {
                id: "missing".into(),
            }],
            ..ChangeSet::default()
        };
        let err = snapshot.apply(&change, &registry).unwrap_err();
        assert!(err.to_string().contains("does not exist for delete"));
    }

    #[test]
    fn apply_rejects_creating_duplicate_node() {
        let registry = registry(None);
        let snapshot = GraphSnapshot::empty();
        let first = snapshot
            .apply(
                &ChangeSet {
                    node_creates: vec![NodeVersion {
                        id: "n1".into(),
                        r#type: Some("A".into()),
                        props: None,
                    }],
                    ..ChangeSet::default()
                },
                &registry,
            )
            .unwrap();
        let err = first
            .apply(
                &ChangeSet {
                    node_creates: vec![NodeVersion {
                        id: "n1".into(),
                        r#type: Some("A".into()),
                        props: None,
                    }],
                    ..ChangeSet::default()
                },
                &registry,
            )
            .unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn apply_rejects_updating_missing_node() {
        let registry = registry(None);
        let snapshot = GraphSnapshot::empty();
        let err = snapshot
            .apply(
                &ChangeSet {
                    node_updates: vec![NodeVersion {
                        id: "missing".into(),
                        r#type: Some("A".into()),
                        props: None,
                    }],
                    ..ChangeSet::default()
                },
                &registry,
            )
            .unwrap_err();
        assert!(err.to_string().contains("missing for update"));
    }

    #[test]
    fn apply_rejects_deleting_missing_edge() {
        let registry = registry(None);
        let snapshot = GraphSnapshot::empty();
        let snapshot = snapshot
            .apply(
                &ChangeSet {
                    node_creates: vec![
                        NodeVersion {
                            id: "a".into(),
                            r#type: Some("A".into()),
                            props: None,
                        },
                        NodeVersion {
                            id: "b".into(),
                            r#type: Some("B".into()),
                            props: None,
                        },
                    ],
                    ..ChangeSet::default()
                },
                &registry,
            )
            .unwrap();
        let err = snapshot
            .apply(
                &ChangeSet {
                    edge_deletes: vec![EdgeTombstone {
                        from: "a".into(),
                        to: "b".into(),
                    }],
                    ..ChangeSet::default()
                },
                &registry,
            )
            .unwrap_err();
        assert!(err.to_string().contains("does not exist for delete"));
    }

    #[test]
    fn apply_rejects_edge_missing_type() {
        let registry = registry(None);
        let snapshot = GraphSnapshot::empty();
        let snapshot = snapshot
            .apply(
                &ChangeSet {
                    node_creates: vec![
                        NodeVersion {
                            id: "a".into(),
                            r#type: Some("A".into()),
                            props: None,
                        },
                        NodeVersion {
                            id: "b".into(),
                            r#type: Some("B".into()),
                            props: None,
                        },
                    ],
                    ..ChangeSet::default()
                },
                &registry,
            )
            .unwrap();
        let err = snapshot
            .apply(
                &ChangeSet {
                    edge_creates: vec![EdgeVersion {
                        id: Some("e1".into()),
                        from: "a".into(),
                        to: "b".into(),
                        r#type: None,
                        directed: None,
                        props: None,
                    }],
                    ..ChangeSet::default()
                },
                &registry,
            )
            .unwrap_err();
        assert!(err.to_string().contains("missing relationship type"));
    }

    #[test]
    fn apply_rejects_duplicate_relationships_when_disallowed() {
        let registry = registry(Some(false));
        let snapshot = GraphSnapshot::empty();
        let change = ChangeSet {
            node_creates: vec![
                NodeVersion {
                    id: "a".into(),
                    r#type: Some("A".into()),
                    props: None,
                },
                NodeVersion {
                    id: "b".into(),
                    r#type: Some("B".into()),
                    props: None,
                },
            ],
            edge_creates: vec![
                EdgeVersion {
                    id: Some("e1".into()),
                    from: "a".into(),
                    to: "b".into(),
                    r#type: Some("rel".into()),
                    directed: None,
                    props: None,
                },
                EdgeVersion {
                    id: Some("e2".into()),
                    from: "a".into(),
                    to: "b".into(),
                    r#type: Some("rel".into()),
                    directed: None,
                    props: None,
                },
            ],
            ..ChangeSet::default()
        };
        let err = snapshot.apply(&change, &registry).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn diff_reports_adds_mods_and_deletes() {
        let registry = registry(None);
        let base = GraphSnapshot::empty();
        let created = base
            .apply(
                &ChangeSet {
                    node_creates: vec![NodeVersion {
                        id: "n1".into(),
                        r#type: Some("A".into()),
                        props: None,
                    }],
                    ..ChangeSet::default()
                },
                &registry,
            )
            .unwrap();

        let modified = created
            .apply(
                &ChangeSet {
                    node_updates: vec![NodeVersion {
                        id: "n1".into(),
                        r#type: Some("A".into()),
                        props: Some(serde_json::json!({"k":"v"})),
                    }],
                    node_creates: vec![NodeVersion {
                        id: "n2".into(),
                        r#type: Some("A".into()),
                        props: None,
                    }],
                    ..ChangeSet::default()
                },
                &registry,
            )
            .unwrap();

        let patch = created.diff(&modified);
        assert_eq!(patch.node_adds.len(), 1);
        assert_eq!(patch.node_mods.len(), 1);

        let deleted = modified
            .apply(
                &ChangeSet {
                    node_deletes: vec![NodeTombstone { id: "n2".into() }],
                    ..ChangeSet::default()
                },
                &registry,
            )
            .unwrap();
        let patch = modified.diff(&deleted);
        assert_eq!(patch.node_dels.len(), 1);
    }
}
