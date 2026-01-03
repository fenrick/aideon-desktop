use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use aideon_mneme::{create_datastore, datastore_path};
use aideon_praxis::temporal::{ChangeSet, CommitSummary};
use aideon_praxis::{
    BaselineDataset, GraphSnapshot, MemoryStore, MetaModelRegistry, PersistedCommit, PraxisEngine,
    PraxisEngineConfig, SqliteDb, Store,
};
use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::MigrateState(args) => migrate_state(args).await,
        Command::ImportDataset(args) => import_dataset(args).await,
        Command::Health(args) => check_health(args).await,
        Command::IpcManifest(args) => export_ipc_manifest(args).await,
    }
}

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Developer utilities for the Aideon Desktop repo"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Convert a legacy in-memory commit export into the durable file-backed store.
    MigrateState(MigrateStateArgs),
    /// Apply the baseline dataset to a datastore (or dry-run for validation).
    ImportDataset(ImportDatasetArgs),
    /// Validate datastore integrity by scanning commits, heads, and snapshots.
    Health(HealthArgs),
    /// Generate a manifest of host IPC command names (for contract tests).
    IpcManifest(IpcManifestArgs),
}

#[derive(Parser)]
struct MigrateStateArgs {
    /// Path to the legacy JSON export containing commits and branches.
    #[arg(long)]
    input: PathBuf,
    /// Destination directory for the new `.praxis` store.
    #[arg(long)]
    output: PathBuf,
    /// Remove any existing data under the output directory before migrating.
    #[arg(long, default_value_t = false)]
    force: bool,
}

#[derive(Parser)]
struct ImportDatasetArgs {
    /// Path to the dataset YAML file (defaults to the checked-in baseline).
    #[arg(long, default_value = "docs/data/base/baseline.yaml")]
    dataset: PathBuf,
    /// Directory where the datastore (sqlite + state file) lives.
    #[arg(long, default_value = ".praxis")]
    datastore: PathBuf,
    /// Validate without writing commits.
    #[arg(long, default_value_t = false)]
    dry_run: bool,
    /// Remove any existing datastore contents before importing.
    #[arg(long, default_value_t = false)]
    force: bool,
}

#[derive(Parser)]
struct HealthArgs {
    /// Directory where the datastore lives (contains datastore.json and sqlite file).
    #[arg(long, default_value = ".praxis")]
    datastore: PathBuf,
    /// Limit the health scan to a single branch.
    #[arg(long)]
    branch: Option<String>,
    /// Reduce output to errors only.
    #[arg(long, default_value_t = false)]
    quiet: bool,
}

#[derive(Parser)]
struct IpcManifestArgs {
    /// Path to write the manifest JSON to (relative to repo root by default).
    #[arg(long, default_value = "docs/contracts/ipc-manifest.json")]
    out: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct IpcManifest {
    schema_version: u32,
    commands: Vec<String>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn collect_rs_files(dir: &PathBuf, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            collect_rs_files(&path, out)?;
            continue;
        }
        if metadata.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn extract_tauri_renames(source: &str) -> Vec<String> {
    fn parse_attr(attr: &str) -> Option<String> {
        let rename_idx = attr.find("rename")?;
        let after = &attr[rename_idx..];
        let first_quote = after.find('"')?;
        let rest = &after[first_quote + 1..];
        let second_quote = rest.find('"')?;
        Some(rest[..second_quote].to_string())
    }

    let mut found = Vec::new();
    let mut buffer: Option<String> = None;
    for line in source.lines() {
        if let Some(current) = buffer.as_mut() {
            current.push_str(line);
            if line.contains(']') {
                if current.contains("#[tauri::command") && current.contains("rename") {
                    if let Some(rename) = parse_attr(current) {
                        found.push(rename);
                    }
                }
                buffer = None;
            }
            continue;
        }

        let trimmed = line.trim();
        if !trimmed.contains("#[tauri::command") {
            continue;
        }
        if trimmed.contains(']') {
            if trimmed.contains("rename") {
                if let Some(rename) = parse_attr(trimmed) {
                    found.push(rename);
                }
            }
            continue;
        }
        buffer = Some(trimmed.to_string());
    }
    found
}

fn build_ipc_manifest() -> Result<IpcManifest> {
    let mut files = Vec::new();
    let desktop_src = repo_root().join("crates/desktop/src");
    collect_rs_files(&desktop_src, &mut files)?;

    let mut commands = std::collections::BTreeSet::<String>::new();
    for path in files {
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("read {}", path.display()))?;
        for rename in extract_tauri_renames(&contents) {
            commands.insert(rename);
        }
    }

    Ok(IpcManifest {
        schema_version: 1,
        commands: commands.into_iter().collect(),
    })
}

async fn export_ipc_manifest(args: IpcManifestArgs) -> Result<()> {
    let repo = repo_root();
    let out = if args.out.is_absolute() {
        args.out
    } else {
        repo.join(args.out)
    };
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create_dir_all {}", parent.display()))?;
    }
    let manifest = build_ipc_manifest()?;
    let json = serde_json::to_string_pretty(&manifest)?;
    fs::write(&out, format!("{json}\n"))
        .with_context(|| format!("write {}", out.display()))?;
    println!("Wrote IPC manifest to {}", out.display());
    Ok(())
}

async fn migrate_state(args: MigrateStateArgs) -> Result<()> {
    let raw = fs::read_to_string(&args.input)
        .with_context(|| format!("failed to read {}", args.input.display()))?;
    let legacy: LegacyState = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", args.input.display()))?;

    if args.output.exists() {
        if args.force {
            fs::remove_dir_all(&args.output)
                .with_context(|| format!("failed to clean {}", args.output.display()))?;
        } else if args.output.join("praxis.sqlite").exists() {
            return Err(anyhow!(
                "output '{}' already contains praxis.sqlite; rerun with --force to overwrite",
                args.output.display()
            ));
        }
    }

    fs::create_dir_all(&args.output)
        .with_context(|| format!("failed to create {}", args.output.display()))?;

    let db_path = args.output.join("praxis.sqlite");
    if db_path.exists() && args.force {
        fs::remove_file(&db_path)
            .with_context(|| format!("failed to remove {}", db_path.display()))?;
    }
    let db = SqliteDb::open(&db_path)
        .await
        .map_err(|err| anyhow!(err.to_string()))?;

    let mut snapshots: HashMap<String, GraphSnapshot> = HashMap::new();
    let mut last_commit_id: Option<String> = None;

    let registry = MetaModelRegistry::embedded().map_err(|err| anyhow!(err.to_string()))?;

    for commit in legacy.commits {
        let base = match commit.summary.parents.first() {
            Some(parent) => snapshots.get(parent).cloned().ok_or_else(|| {
                anyhow!("missing parent '{parent}' for commit {}", commit.summary.id)
            })?,
            None => GraphSnapshot::empty(),
        };
        let next = base
            .apply(&commit.change_set, &registry)
            .map_err(|err| anyhow!("apply commit {} failed: {err}", commit.summary.id))?;

        let persisted = PersistedCommit {
            summary: commit.summary.clone(),
            change_set: commit.change_set.clone(),
        };
        db.put_commit(&persisted)
            .await
            .map_err(|err| anyhow!(err.to_string()))?;

        last_commit_id = Some(persisted.summary.id.clone());
        snapshots.insert(persisted.summary.id, next);
    }

    if legacy.branches.is_empty() {
        db.ensure_branch("main")
            .await
            .map_err(|err| anyhow!(err.to_string()))?;
        db.compare_and_swap_branch("main", None, last_commit_id.as_deref())
            .await
            .map_err(|err| anyhow!(err.to_string()))?;
    } else {
        for branch in legacy.branches {
            db.ensure_branch(&branch.name)
                .await
                .map_err(|err| anyhow!(err.to_string()))?;
            db.compare_and_swap_branch(&branch.name, None, branch.head.as_deref())
                .await
                .map_err(|err| anyhow!(err.to_string()))?;
        }
    }

    println!(
        "Migrated {} commits into {}",
        snapshots.len(),
        args.output.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_health_defaults() {
        let cli = Cli::parse_from(["xtask", "health"]);
        match cli.command {
            Command::Health(args) => {
                assert_eq!(args.datastore, PathBuf::from(".praxis"));
                assert!(args.branch.is_none());
                assert!(!args.quiet);
            }
            _ => panic!("expected health command"),
        }
    }

    #[test]
    fn cli_parses_import_dataset_flags() {
        let cli = Cli::parse_from([
            "xtask",
            "import-dataset",
            "--dataset",
            "path/to/data.yaml",
            "--datastore",
            "/tmp/praxis",
            "--dry-run",
            "--force",
        ]);
        match cli.command {
            Command::ImportDataset(args) => {
                assert_eq!(args.dataset, PathBuf::from("path/to/data.yaml"));
                assert_eq!(args.datastore, PathBuf::from("/tmp/praxis"));
                assert!(args.dry_run);
                assert!(args.force);
            }
            _ => panic!("expected import-dataset command"),
        }
    }

    #[test]
    fn ipc_manifest_is_deterministic_and_checked_in() {
        let repo = repo_root();
        let manifest = build_ipc_manifest().expect("build manifest");
        let path = repo.join("docs/contracts/ipc-manifest.json");
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("missing {}; run `cargo run -p aideon_xtask -- ipc-manifest` to generate it", path.display()));
        let on_disk: IpcManifest =
            serde_json::from_str(&raw).expect("parse ipc-manifest.json");
        assert_eq!(
            on_disk, manifest,
            "ipc-manifest.json drifted; rerun `cargo run -p aideon_xtask -- ipc-manifest`"
        );
    }
}

async fn import_dataset(args: ImportDatasetArgs) -> Result<()> {
    let dataset = BaselineDataset::from_path(&args.dataset)
        .or_else(|_| BaselineDataset::embedded())
        .map_err(|err| anyhow!(err.to_string()))?;

    if args.dry_run {
        dry_run_dataset(&dataset).await?;
        return Ok(());
    }

    if args.force && args.datastore.exists() {
        fs::remove_dir_all(&args.datastore)
            .with_context(|| format!("failed to clean {}", args.datastore.display()))?;
    }

    let db_path =
        create_datastore(&args.datastore, None).map_err(|err| anyhow!(err.to_string()))?;
    let storage = SqliteDb::open(&db_path)
        .await
        .map_err(|err| anyhow!(err.to_string()))?;
    let engine = PraxisEngine::with_stores_unseeded(
        PraxisEngineConfig::default(),
        Arc::new(storage.clone()),
    )
    .await
    .map_err(|err| anyhow!(err.to_string()))?;

    let has_commits = engine
        .list_branches()
        .await
        .into_iter()
        .any(|branch| branch.name == "main" && branch.head.is_some());
    if has_commits {
        return Err(anyhow!(
            "datastore '{}' already contains commits; rerun with --force",
            args.datastore.display()
        ));
    }

    engine
        .bootstrap_with_dataset(&dataset)
        .await
        .map_err(|err| anyhow!(err.to_string()))?;

    let commits = engine
        .list_commits("main".into())
        .await
        .map_err(|err| anyhow!(err.to_string()))?;
    if let Some(last) = commits.last() {
        let stats = engine
            .stats_for_commit(&last.id)
            .await
            .map_err(|err| anyhow!(err.to_string()))?;
        println!(
            "imported dataset {} (commits={} nodes={} edges={}) into {}",
            dataset.version,
            commits.len(),
            stats.node_count,
            stats.edge_count,
            db_path.display()
        );
    } else {
        println!(
            "imported dataset {} (no commits reported) into {}",
            dataset.version,
            db_path.display()
        );
    }

    Ok(())
}

async fn check_health(args: HealthArgs) -> Result<()> {
    let db_path = datastore_path(&args.datastore)
        .with_context(|| format!("resolve datastore under {}", args.datastore.display()))?;
    let storage = SqliteDb::open(&db_path)
        .await
        .map_err(|err| anyhow!(err.to_string()))?;
    let engine = PraxisEngine::with_stores_unseeded(
        PraxisEngineConfig::default(),
        Arc::new(storage.clone()),
    )
    .await
    .map_err(|err| anyhow!(err.to_string()))?;

    let branches = engine.list_branches().await;
    let filtered: Vec<_> = branches
        .into_iter()
        .filter(|branch| match &args.branch {
            Some(target) => &branch.name == target,
            None => true,
        })
        .collect();

    if filtered.is_empty() {
        return Err(anyhow!("no branches found (filter={:?})", args.branch));
    }

    struct Finding {
        kind: &'static str,
        message: String,
    }

    let mut findings: Vec<Finding> = Vec::new();
    let mut commit_total: usize = 0;

    for branch in &filtered {
        let branch_name = branch.name.clone();
        let commits = engine
            .list_commits(branch_name.clone())
            .await
            .map_err(|err| anyhow!(err.to_string()))?;
        commit_total += commits.len();

        match (branch.head.as_ref(), commits.last()) {
            (Some(head), Some(last)) if head != &last.id => {
                findings.push(Finding {
                    kind: "error",
                    message: format!(
                        "branch '{}' head {} mismatches latest commit {}",
                        branch_name, head, last.id
                    ),
                });
            }
            (Some(head), None) => findings.push(Finding {
                kind: "error",
                message: format!(
                    "branch '{}' records head {} but has no commits",
                    branch_name, head
                ),
            }),
            (None, Some(last)) => findings.push(Finding {
                kind: "warning",
                message: format!(
                    "branch '{}' has {} commits but no recorded head (latest {})",
                    branch_name,
                    commits.len(),
                    last.id
                ),
            }),
            (None, None) => findings.push(Finding {
                kind: "warning",
                message: format!("branch '{}' is empty", branch_name),
            }),
            _ => {}
        }

        if let Some(last) = commits.last()
            && last.time.is_none()
        {
            findings.push(Finding {
                kind: "warning",
                message: format!(
                    "branch '{}' head {} missing timestamp metadata",
                    branch_name, last.id
                ),
            });
        }

        for commit in commits {
            if let Err(err) = engine.stats_for_commit(&commit.id).await {
                findings.push(Finding {
                    kind: "error",
                    message: format!("snapshot for commit {} unreadable: {}", commit.id, err),
                });
            }

            let tag_key = format!("snapshot/{}", commit.id);
            match storage.get_tag(&tag_key).await {
                Ok(Some(resolved)) if resolved == commit.id => {}
                Ok(Some(resolved)) => findings.push(Finding {
                    kind: "error",
                    message: format!(
                        "snapshot tag {} points to {} instead of {}",
                        tag_key, resolved, commit.id
                    ),
                }),
                Ok(None) => findings.push(Finding {
                    kind: "warning",
                    message: format!("snapshot tag missing for commit {}", commit.id),
                }),
                Err(err) => findings.push(Finding {
                    kind: "error",
                    message: format!("snapshot tag lookup failed for {}: {}", commit.id, err),
                }),
            }
        }
    }

    if !args.quiet {
        println!("Datastore: {}", db_path.display());
        println!("Branches scanned: {}", filtered.len());
        println!("Commits scanned: {}", commit_total);
    }

    let errors: Vec<&Finding> = findings.iter().filter(|f| f.kind == "error").collect();
    let warnings: Vec<&Finding> = findings.iter().filter(|f| f.kind == "warning").collect();

    if !args.quiet {
        for warn in &warnings {
            println!("warning: {}", warn.message);
        }
    }

    if !errors.is_empty() {
        for err in &errors {
            eprintln!("error: {}", err.message);
        }
        return Err(anyhow!(
            "health check failed ({} errors, {} warnings)",
            errors.len(),
            warnings.len()
        ));
    }

    if !args.quiet {
        println!("health check passed with {} warnings", warnings.len());
    }
    Ok(())
}

async fn dry_run_dataset(dataset: &BaselineDataset) -> Result<()> {
    let store: Arc<dyn Store> = Arc::new(MemoryStore::default());
    let engine = PraxisEngine::with_stores_unseeded(PraxisEngineConfig::default(), store)
        .await
        .map_err(|err| anyhow!(err.to_string()))?;
    engine
        .bootstrap_with_dataset(dataset)
        .await
        .map_err(|err| anyhow!(err.to_string()))?;

    let commits = engine
        .list_commits("main".into())
        .await
        .map_err(|err| anyhow!(err.to_string()))?;
    let stats = if let Some(summary) = commits.last() {
        engine
            .stats_for_commit(&summary.id)
            .await
            .map_err(|err| anyhow!(err.to_string()))?
    } else {
        Default::default()
    };
    println!(
        "dry-run ok: dataset {} commits={} nodes={} edges={}",
        dataset.version,
        commits.len(),
        stats.node_count,
        stats.edge_count
    );
    Ok(())
}

#[derive(serde::Deserialize)]
struct LegacyState {
    commits: Vec<LegacyCommit>,
    #[serde(default)]
    branches: Vec<LegacyBranch>,
}

#[derive(serde::Deserialize)]
struct LegacyCommit {
    summary: CommitSummary,
    #[serde(default)]
    change_set: ChangeSet,
}

#[derive(serde::Deserialize)]
struct LegacyBranch {
    name: String,
    #[serde(default)]
    head: Option<String>,
}
