//! Praxis API-facing DTOs and Tauri command handlers bridging React IPC calls.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::ipc::HostError;

#[tauri::command]
pub async fn praxis_graph_view(
    definition: GraphViewDefinition,
) -> Result<GraphViewModel, HostError> {
    Ok(GraphViewModel::demo(definition))
}

#[tauri::command]
pub async fn praxis_catalogue_view(
    definition: CatalogueViewDefinition,
) -> Result<CatalogueViewModel, HostError> {
    Ok(CatalogueViewModel::demo(definition))
}

#[tauri::command]
pub async fn praxis_matrix_view(
    definition: MatrixViewDefinition,
) -> Result<MatrixViewModel, HostError> {
    Ok(MatrixViewModel::demo(definition))
}

#[allow(dead_code)]
#[tauri::command]
pub async fn praxis_chart_view(
    definition: ChartViewDefinition,
) -> Result<ChartViewModel, HostError> {
    Ok(ChartViewModel::demo(definition))
}

#[tauri::command]
pub async fn praxis_apply_operations(
    operations: Vec<PraxisOperation>,
) -> Result<OperationBatchResult, HostError> {
    if operations.is_empty() {
        return Ok(OperationBatchResult::rejected("no operations supplied"));
    }
    Ok(OperationBatchResult::accepted(next_commit_id()))
}

#[tauri::command]
pub async fn praxis_list_scenarios() -> Result<Vec<ScenarioSummary>, HostError> {
    Ok(ScenarioSummary::demo_list())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphViewDefinition {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub as_of: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<ViewFilters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<GraphViewScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphViewScope {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub root_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueViewDefinition {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub as_of: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<ViewFilters>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub columns: Vec<CatalogueColumn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixViewDefinition {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub as_of: String,
    pub row_type: String,
    pub column_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<ViewFilters>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartViewDefinition {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub as_of: String,
    pub chart_type: String,
    pub measure: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimension: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<ViewFilters>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwinNode {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TwinEdge {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNodeView {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdgeView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewMetadata {
    pub id: String,
    pub name: String,
    pub as_of: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenario: Option<String>,
    pub fetched_at: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewStats {
    pub nodes: usize,
    pub edges: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphViewModel {
    pub metadata: ViewMetadata,
    pub stats: ViewStats,
    pub nodes: Vec<GraphNodeView>,
    pub edges: Vec<GraphEdgeView>,
}

impl GraphViewModel {
    fn demo(definition: GraphViewDefinition) -> Self {
        let nodes = vec![
            GraphNodeView {
                id: "cap-customer-onboarding".into(),
                label: "Customer Onboarding".into(),
                r#type: Some("Capability".into()),
                position: Some(Position { x: 120.0, y: 200.0 }),
                props: None,
            },
            GraphNodeView {
                id: "cap-customer-support".into(),
                label: "Customer Support".into(),
                r#type: Some("Capability".into()),
                position: Some(Position { x: 420.0, y: 120.0 }),
                props: None,
            },
            GraphNodeView {
                id: "app-workflow".into(),
                label: "Workflow Engine".into(),
                r#type: Some("Application".into()),
                position: Some(Position { x: 420.0, y: 320.0 }),
                props: None,
            },
            GraphNodeView {
                id: "svc-auth".into(),
                label: "Identity Service".into(),
                r#type: Some("Service".into()),
                position: Some(Position { x: 680.0, y: 220.0 }),
                props: None,
            },
        ];
        let edges = vec![
            GraphEdgeView {
                id: Some("edge-1".into()),
                from: nodes[0].id.clone(),
                to: nodes[1].id.clone(),
                r#type: Some("supports".into()),
                label: Some("handoff".into()),
                props: None,
            },
            GraphEdgeView {
                id: Some("edge-2".into()),
                from: nodes[1].id.clone(),
                to: nodes[2].id.clone(),
                r#type: Some("depends_on".into()),
                label: Some("tickets".into()),
                props: None,
            },
            GraphEdgeView {
                id: Some("edge-3".into()),
                from: nodes[2].id.clone(),
                to: nodes[3].id.clone(),
                r#type: Some("depends_on".into()),
                label: Some("auth".into()),
                props: None,
            },
        ];
        Self {
            metadata: metadata_from(
                &definition.id,
                &definition.name,
                &definition.as_of,
                definition.scenario.clone(),
            ),
            stats: ViewStats {
                nodes: nodes.len(),
                edges: edges.len(),
            },
            nodes,
            edges,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueColumn {
    pub id: String,
    pub label: String,
    pub r#type: CatalogueColumnType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CatalogueColumnType {
    String,
    Number,
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueRow {
    pub id: String,
    pub values: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueViewModel {
    pub metadata: ViewMetadata,
    pub columns: Vec<CatalogueColumn>,
    pub rows: Vec<CatalogueRow>,
}

impl CatalogueViewModel {
    fn demo(definition: CatalogueViewDefinition) -> Self {
        let cols = if definition.columns.is_empty() {
            vec![
                CatalogueColumn {
                    id: "name".into(),
                    label: "Name".into(),
                    r#type: CatalogueColumnType::String,
                },
                CatalogueColumn {
                    id: "owner".into(),
                    label: "Owner".into(),
                    r#type: CatalogueColumnType::String,
                },
                CatalogueColumn {
                    id: "state".into(),
                    label: "State".into(),
                    r#type: CatalogueColumnType::String,
                },
            ]
        } else {
            definition.columns
        };
        let rows = vec![
            CatalogueRow {
                id: "cap-customer-onboarding".into(),
                values: map_from(
                    json!({ "name": "Customer Onboarding", "owner": "CX", "state": "Pilot" }),
                ),
            },
            CatalogueRow {
                id: "cap-customer-support".into(),
                values: map_from(
                    json!({ "name": "Customer Support", "owner": "Ops", "state": "Production" }),
                ),
            },
            CatalogueRow {
                id: "cap-incident-response".into(),
                values: map_from(
                    json!({ "name": "Incident Response", "owner": "SRE", "state": "In Flight" }),
                ),
            },
        ];
        Self {
            metadata: metadata_from(
                &definition.id,
                &definition.name,
                &definition.as_of,
                definition.scenario.clone(),
            ),
            columns: cols,
            rows,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixAxis {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MatrixCellState {
    Connected,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixCell {
    pub row_id: String,
    pub column_id: String,
    pub state: MatrixCellState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strength: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixViewModel {
    pub metadata: ViewMetadata,
    pub rows: Vec<MatrixAxis>,
    pub columns: Vec<MatrixAxis>,
    pub cells: Vec<MatrixCell>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartPoint {
    pub label: String,
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartSeries {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub points: Vec<ChartPoint>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartKpiSummary {
    pub value: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trend: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartViewModel {
    pub metadata: ViewMetadata,
    pub chart_type: String,
    pub series: Vec<ChartSeries>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kpi: Option<ChartKpiSummary>,
}

impl MatrixViewModel {
    fn demo(definition: MatrixViewDefinition) -> Self {
        let rows = vec![
            MatrixAxis {
                id: "cap-customer-onboarding".into(),
                label: "Customer Onboarding".into(),
            },
            MatrixAxis {
                id: "cap-incident-response".into(),
                label: "Incident Response".into(),
            },
        ];
        let cols = vec![
            MatrixAxis {
                id: "svc-auth".into(),
                label: "Identity Service".into(),
            },
            MatrixAxis {
                id: "svc-search".into(),
                label: "Search Platform".into(),
            },
        ];
        let cells = vec![
            MatrixCell {
                row_id: rows[0].id.clone(),
                column_id: cols[0].id.clone(),
                state: MatrixCellState::Connected,
                strength: Some(0.8),
                value: None,
            },
            MatrixCell {
                row_id: rows[0].id.clone(),
                column_id: cols[1].id.clone(),
                state: MatrixCellState::Missing,
                strength: None,
                value: None,
            },
            MatrixCell {
                row_id: rows[1].id.clone(),
                column_id: cols[0].id.clone(),
                state: MatrixCellState::Connected,
                strength: Some(0.4),
                value: None,
            },
            MatrixCell {
                row_id: rows[1].id.clone(),
                column_id: cols[1].id.clone(),
                state: MatrixCellState::Missing,
                strength: None,
                value: None,
            },
        ];
        Self {
            metadata: metadata_from(
                &definition.id,
                &definition.name,
                &definition.as_of,
                definition.scenario.clone(),
            ),
            rows,
            columns: cols,
            cells,
        }
    }
}

#[allow(dead_code)]
impl ChartViewModel {
    fn demo(definition: ChartViewDefinition) -> Self {
        let metadata = metadata_from(
            &definition.id,
            &definition.name,
            &definition.as_of,
            definition.scenario.clone(),
        );
        match definition.chart_type.as_str() {
            "kpi" => Self {
                metadata,
                chart_type: "kpi".into(),
                series: Vec::new(),
                kpi: Some(ChartKpiSummary {
                    value: 128.0,
                    units: Some("services".into()),
                    delta: Some(6.0),
                    trend: Some("up".into()),
                }),
            },
            "line" => Self {
                metadata,
                chart_type: "line".into(),
                series: vec![ChartSeries {
                    id: "velocity".into(),
                    label: "Delivery velocity".into(),
                    color: Some("#2563eb".into()),
                    points: recent_velocity_points(),
                }],
                kpi: None,
            },
            _ => Self {
                metadata,
                chart_type: "bar".into(),
                series: vec![
                    ChartSeries {
                        id: "current".into(),
                        label: "Current".into(),
                        color: Some("#0f172a".into()),
                        points: competency_scores(),
                    },
                    ChartSeries {
                        id: "target".into(),
                        label: "Target".into(),
                        color: Some("#10b981".into()),
                        points: competency_targets(),
                    },
                ],
                kpi: None,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PraxisOperation {
    CreateNode { node: TwinNode },
    UpdateNode { node: TwinNode },
    DeleteNode { node_id: String },
    CreateEdge { edge: TwinEdge },
    DeleteEdge { edge_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationBatchResult {
    pub accepted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_id: Option<String>,
}

impl OperationBatchResult {
    fn accepted(commit_id: String) -> Self {
        Self {
            accepted: true,
            message: Some("mock commit created".into()),
            commit_id: Some(commit_id),
        }
    }

    fn rejected(message: &str) -> Self {
        Self {
            accepted: false,
            message: Some(message.into()),
            commit_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScenarioSummary {
    pub id: String,
    pub name: String,
    pub branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
}

impl ScenarioSummary {
    pub(crate) fn demo_list() -> Vec<Self> {
        vec![
            ScenarioSummary {
                id: "scenario-main".into(),
                name: "Mainline FY25".into(),
                branch: "main".into(),
                description: Some("Authoritative branch".into()),
                updated_at: now_iso(),
                is_default: Some(true),
            },
            ScenarioSummary {
                id: "scenario-chrona".into(),
                name: "Chrona Playground".into(),
                branch: "chronaplay".into(),
                description: Some("Prototype overlays".into()),
                updated_at: now_iso(),
                is_default: None,
            },
        ]
    }
}

fn metadata_from(id: &str, name: &str, as_of: &str, scenario: Option<String>) -> ViewMetadata {
    ViewMetadata {
        id: id.into(),
        name: name.into(),
        as_of: as_of.into(),
        scenario,
        fetched_at: now_iso(),
        source: "host".into(),
    }
}

fn map_from(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

#[allow(dead_code)]
fn recent_velocity_points() -> Vec<ChartPoint> {
    let now = OffsetDateTime::now_utc();
    let mut points = Vec::new();
    for index in 0..7 {
        let offset = 6 - index;
        if let Some(timestamp) = now.checked_sub(time::Duration::days(offset)) {
            points.push(ChartPoint {
                label: timestamp.weekday().to_string(),
                value: 78.0 + (index as f64 * 3.5),
                timestamp: Some(timestamp.format(&Rfc3339).unwrap_or_else(|_| now_iso())),
            });
        }
    }
    points
}

#[allow(dead_code)]
fn competency_scores() -> Vec<ChartPoint> {
    ["Security", "Resilience", "Efficiency", "Experience"]
        .iter()
        .map(|label| ChartPoint {
            label: (*label).into(),
            value: seeded_score(label) as f64,
            timestamp: None,
        })
        .collect()
}

#[allow(dead_code)]
fn competency_targets() -> Vec<ChartPoint> {
    ["Security", "Resilience", "Efficiency", "Experience"]
        .iter()
        .map(|label| ChartPoint {
            label: (*label).into(),
            value: 95.0,
            timestamp: None,
        })
        .collect()
}

#[allow(dead_code)]
fn seeded_score(label: &str) -> i32 {
    let mut hash = 0i32;
    for ch in label.chars() {
        let code_point = ch as i32;
        hash = hash.wrapping_shl(5).wrapping_sub(hash) + code_point;
    }
    60 + (hash.abs() % 35)
}

fn now_iso() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn next_commit_id() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(1);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("mock-commit-{:04}", id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::OffsetDateTime;

    #[test]
    fn metadata_contains_source() {
        let metadata = metadata_from("view-1", "Test", "2025-11-14", Some("main".into()));
        assert_eq!(metadata.source, "host");
        assert_eq!(metadata.scenario.as_deref(), Some("main"));
        assert!(OffsetDateTime::parse(&metadata.fetched_at, &Rfc3339).is_ok());
    }

    #[test]
    fn scenarios_have_default() {
        let summary = ScenarioSummary::demo_list();
        assert!(summary.iter().any(|s| s.is_default == Some(true)));
    }

    #[tokio::test]
    async fn apply_operations_rejects_empty_payload() {
        let out = praxis_apply_operations(Vec::new()).await.unwrap();
        assert!(!out.accepted);
        assert_eq!(out.commit_id, None);
        assert!(out.message.unwrap_or_default().contains("no operations"));
    }

    #[tokio::test]
    async fn apply_operations_accepts_non_empty_payload_and_increments_commit_id() {
        let op = PraxisOperation::CreateNode {
            node: TwinNode {
                id: "n1".into(),
                r#type: Some("Capability".into()),
                props: None,
            },
        };
        let first = praxis_apply_operations(vec![op.clone()]).await.unwrap();
        let second = praxis_apply_operations(vec![op]).await.unwrap();
        assert!(first.accepted);
        assert!(second.accepted);
        assert!(
            first
                .commit_id
                .as_deref()
                .unwrap_or_default()
                .starts_with("mock-commit-")
        );
        assert_ne!(first.commit_id, second.commit_id);
    }

    #[test]
    fn graph_view_demo_is_well_formed() {
        let def = GraphViewDefinition {
            id: "graph-1".into(),
            name: "Graph".into(),
            kind: "graph".into(),
            as_of: "2025-12-01".into(),
            scenario: None,
            confidence: Some(0.9),
            filters: Some(ViewFilters {
                node_types: Some(vec!["Capability".into()]),
                edge_types: None,
                tags: None,
                search: None,
            }),
            scope: Some(GraphViewScope {
                root_ids: vec!["cap-customer-onboarding".into()],
            }),
        };
        let model = GraphViewModel::demo(def);
        assert_eq!(model.metadata.id, "graph-1");
        assert_eq!(model.metadata.as_of, "2025-12-01");
        assert_eq!(model.stats.nodes, model.nodes.len());
        assert_eq!(model.stats.edges, model.edges.len());
        assert!(model.nodes.iter().all(|n| !n.id.is_empty()));
        assert!(
            model
                .edges
                .iter()
                .all(|e| !e.from.is_empty() && !e.to.is_empty())
        );
    }

    #[test]
    fn catalogue_view_demo_defaults_columns_when_missing() {
        let def = CatalogueViewDefinition {
            id: "cat-1".into(),
            name: "Catalogue".into(),
            kind: "catalogue".into(),
            as_of: "2025-12-01".into(),
            scenario: None,
            confidence: None,
            filters: None,
            columns: Vec::new(),
        };
        let model = CatalogueViewModel::demo(def);
        assert_eq!(model.columns.len(), 3);
        assert!(!model.rows.is_empty());
        assert!(model.rows.iter().all(|row| row.values.contains_key("name")));
    }

    #[test]
    fn catalogue_view_demo_respects_explicit_columns() {
        let def = CatalogueViewDefinition {
            id: "cat-2".into(),
            name: "Catalogue".into(),
            kind: "catalogue".into(),
            as_of: "2025-12-01".into(),
            scenario: None,
            confidence: None,
            filters: None,
            columns: vec![CatalogueColumn {
                id: "owner".into(),
                label: "Owner".into(),
                r#type: CatalogueColumnType::String,
            }],
        };
        let model = CatalogueViewModel::demo(def);
        assert_eq!(model.columns.len(), 1);
        assert_eq!(model.columns[0].id, "owner");
    }

    #[test]
    fn matrix_view_demo_has_cells_for_each_axis_pair() {
        let def = MatrixViewDefinition {
            id: "matrix-1".into(),
            name: "Matrix".into(),
            kind: "matrix".into(),
            as_of: "2025-12-01".into(),
            row_type: "Capability".into(),
            column_type: "Service".into(),
            scenario: None,
            confidence: None,
            filters: None,
        };
        let model = MatrixViewModel::demo(def);
        assert_eq!(model.cells.len(), model.rows.len() * model.columns.len());
        assert!(
            model
                .cells
                .iter()
                .any(|cell| matches!(cell.state, MatrixCellState::Connected))
        );
    }

    #[test]
    fn chart_view_demo_supports_kpi_and_line() {
        let base = ChartViewDefinition {
            id: "chart-1".into(),
            name: "Chart".into(),
            kind: "chart".into(),
            as_of: "2025-12-01".into(),
            chart_type: "kpi".into(),
            measure: "count".into(),
            dimension: None,
            scenario: None,
            confidence: None,
            filters: None,
        };
        let kpi = ChartViewModel::demo(base.clone());
        assert_eq!(kpi.chart_type, "kpi");
        assert!(kpi.kpi.is_some());
        assert!(kpi.series.is_empty());

        let mut line_def = base;
        line_def.chart_type = "line".into();
        let line = ChartViewModel::demo(line_def);
        assert_eq!(line.chart_type, "line");
        assert!(line.kpi.is_none());
        assert_eq!(line.series.len(), 1);
        assert!(!line.series[0].points.is_empty());
    }

    #[test]
    fn map_from_rejects_non_objects() {
        assert!(map_from(Value::Null).is_empty());
        assert!(map_from(Value::Array(vec![])).is_empty());
        let obj = map_from(json!({"a": 1}));
        assert_eq!(obj.get("a"), Some(&json!(1)));
    }

    #[test]
    fn seeded_score_is_deterministic_and_in_range() {
        let a = seeded_score("Security");
        let b = seeded_score("Security");
        assert_eq!(a, b);
        assert!((60..=94).contains(&a));
    }
}
