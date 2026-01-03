use aideon_mneme_store::ops::{CreateEdgeInput, CreateNodeInput};
use aideon_mneme_store::{
    ActorId, AnalyticsApi, AnalyticsResultsApi, EntityKind, GetGraphDegreeStatsInput,
    GetGraphEdgeTypeCountsInput, GetProjectionEdgesInput, GraphWriteApi, Hlc, Id, Layer,
    MetamodelApi, MetamodelBatch, MnemeConfig, MnemeProcessingApi, MnemeStore, PageRankRunSpec,
    PartitionId, PoolConfig, RunWorkerInput, TriggerProcessingInput, TypeDef, ValidTime,
};
use tempfile::tempdir;

fn new_ids() -> (PartitionId, ActorId) {
    (PartitionId(Id::new()), ActorId(Id::new()))
}

#[tokio::test]
async fn degree_stats_and_edge_type_counts_refresh() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let mut config = MnemeConfig::default_sqlite(base.join("analytics.sqlite").to_string_lossy());
    config.pool = Some(PoolConfig {
        max_connections: Some(5),
        min_connections: Some(1),
        connect_timeout_ms: Some(5_000),
        acquire_timeout_ms: Some(5_000),
        idle_timeout_ms: Some(60_000),
    });
    let store = MnemeStore::connect(&config, base).await?;
    let (partition, actor) = new_ids();

    let node_a = Id::new();
    let node_b = Id::new();
    let node_c = Id::new();
    for node_id in [node_a, node_b, node_c] {
        store
            .create_node(CreateNodeInput {
                partition,
                scenario_id: None,
                actor,
                asserted_at: Hlc::now(),
                node_id,
                type_id: None,
                acl_group_id: None,
                owner_actor_id: None,
                visibility: None,
                write_options: None,
            })
            .await?;
    }

    let edge_type_a = Id::new();
    let edge_type_b = Id::new();

    store
        .upsert_metamodel_batch(
            partition,
            actor,
            Hlc::now(),
            MetamodelBatch {
                types: vec![
                    TypeDef {
                        type_id: edge_type_a,
                        applies_to: EntityKind::Edge,
                        label: "edge-type-a".to_string(),
                        is_abstract: false,
                        parent_type_id: None,
                    },
                    TypeDef {
                        type_id: edge_type_b,
                        applies_to: EntityKind::Edge,
                        label: "edge-type-b".to_string(),
                        is_abstract: false,
                        parent_type_id: None,
                    },
                ],
                fields: Vec::new(),
                type_fields: Vec::new(),
                edge_type_rules: Vec::new(),
                metamodel_version: None,
                metamodel_source: None,
            },
        )
        .await?;

    store
        .create_edge(CreateEdgeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            edge_id: Id::new(),
            type_id: Some(edge_type_a),
            src_id: node_a,
            dst_id: node_b,
            exists_valid_from: ValidTime(0),
            exists_valid_to: None,
            layer: Layer::Actual,
            weight: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;
    store
        .create_edge(CreateEdgeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            edge_id: Id::new(),
            type_id: Some(edge_type_a),
            src_id: node_b,
            dst_id: node_c,
            exists_valid_from: ValidTime(0),
            exists_valid_to: None,
            layer: Layer::Actual,
            weight: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;
    store
        .create_edge(CreateEdgeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            edge_id: Id::new(),
            type_id: Some(edge_type_b),
            src_id: node_a,
            dst_id: node_c,
            exists_valid_from: ValidTime(0),
            exists_valid_to: None,
            layer: Layer::Actual,
            weight: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;

    store
        .trigger_refresh_analytics_projections(TriggerProcessingInput {
            partition,
            scenario_id: None,
            reason: "test-refresh".to_string(),
        })
        .await?;
    let processed = store
        .run_processing_worker(RunWorkerInput {
            max_jobs: 10,
            lease_millis: 10_000,
        })
        .await?;
    assert!(processed > 0);

    let stats = store
        .get_graph_degree_stats(GetGraphDegreeStatsInput {
            partition,
            scenario_id: None,
            as_of_valid_time: None,
            entity_ids: None,
            limit: None,
        })
        .await?;

    let mut by_id = stats
        .into_iter()
        .map(|stat| (stat.entity_id, (stat.out_degree, stat.in_degree)))
        .collect::<std::collections::HashMap<_, _>>();

    assert_eq!(by_id.remove(&node_a), Some((2, 0)));
    assert_eq!(by_id.remove(&node_b), Some((1, 1)));
    assert_eq!(by_id.remove(&node_c), Some((0, 2)));

    let counts = store
        .get_graph_edge_type_counts(GetGraphEdgeTypeCountsInput {
            partition,
            scenario_id: None,
            edge_type_ids: None,
            limit: None,
        })
        .await?;
    let mut counts_by_type = counts
        .into_iter()
        .map(|item| (item.edge_type_id, item.count))
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(counts_by_type.remove(&Some(edge_type_a)), Some(2));
    assert_eq!(counts_by_type.remove(&Some(edge_type_b)), Some(1));
    Ok(())
}

#[tokio::test]
async fn pagerank_scores_roundtrip() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("pagerank.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let (partition, actor) = new_ids();

    let node_a = Id::new();
    let node_b = Id::new();
    let run_id = store
        .store_pagerank_scores(
            partition,
            actor,
            None,
            None,
            PageRankRunSpec {
                damping: 0.85,
                max_iters: 20,
                tol: 1e-6,
                personalised_seed: None,
            },
            vec![(node_a, 0.2), (node_b, 0.9)],
        )
        .await?;

    let scores = store.get_pagerank_scores(partition, run_id, 5).await?;
    assert_eq!(scores.first().map(|(id, _)| *id), Some(node_b));
    Ok(())
}

#[tokio::test]
async fn projection_edges_after_refresh_are_visible() -> aideon_mneme_store::MnemeResult<()> {
    let dir = tempdir().expect("tempdir");
    let base = dir.path();
    let config = MnemeConfig::default_sqlite(base.join("projection.sqlite").to_string_lossy());
    let store = MnemeStore::connect(&config, base).await?;
    let (partition, actor) = new_ids();

    let node_a = Id::new();
    let node_b = Id::new();
    store
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id: node_a,
            type_id: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;
    store
        .create_node(CreateNodeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            node_id: node_b,
            type_id: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;

    store
        .create_edge(CreateEdgeInput {
            partition,
            scenario_id: None,
            actor,
            asserted_at: Hlc::now(),
            edge_id: Id::new(),
            type_id: None,
            src_id: node_a,
            dst_id: node_b,
            exists_valid_from: ValidTime(0),
            exists_valid_to: None,
            layer: Layer::Actual,
            weight: None,
            acl_group_id: None,
            owner_actor_id: None,
            visibility: None,
            write_options: None,
        })
        .await?;

    store
        .trigger_refresh_analytics_projections(TriggerProcessingInput {
            partition,
            scenario_id: None,
            reason: "test-refresh".to_string(),
        })
        .await?;
    store
        .run_processing_worker(RunWorkerInput {
            max_jobs: 10,
            lease_millis: 10_000,
        })
        .await?;

    let edges = store
        .get_projection_edges(GetProjectionEdgesInput {
            partition,
            scenario_id: None,
            security_context: None,
            at_valid_time: Some(ValidTime(1)),
            as_of_asserted_at: None,
            edge_type_filter: None,
            limit: None,
        })
        .await?;
    assert_eq!(edges.len(), 1);
    Ok(())
}
