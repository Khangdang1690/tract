//! `tractd` — the tract daemon entrypoint.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use clap::{Parser, Subcommand};
use tract_fetcher::{Cache, HttpClient};
use tract_proto::{FetchOptions, Method, ResultBody};

use tractd::{client, ipc, orchestrator::Orchestrator, server, state::AppState};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(name = "tractd", version, about = "tract daemon")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run the daemon in the foreground.
    Serve,
    /// Connect to a running daemon and print status.
    Status,
    /// One-shot fetch via an in-process pipeline (debugging).
    Fetch {
        url: String,
        #[arg(long)]
        force_refresh: bool,
        /// Print the full FetchResult as JSON instead of just the markdown.
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Serve => run_serve().await,
        Cmd::Status => run_status().await,
        Cmd::Fetch {
            url,
            force_refresh,
            json,
        } => run_fetch(url, force_refresh, json).await,
    }
}

fn build_orchestrator() -> anyhow::Result<(Orchestrator, PathBuf)> {
    let data_dir = ipc::default_data_dir()?;
    std::fs::create_dir_all(&data_dir)
        .with_context(|| format!("creating data dir {}", data_dir.display()))?;
    let cache_path = data_dir.join("cache.db");
    let cache = Cache::open(&cache_path)
        .with_context(|| format!("opening cache at {}", cache_path.display()))?;
    let profile = Arc::new(tract_profile::Profile::default());
    let http = HttpClient::new(&profile.http).context("building HTTP client")?;
    Ok((Orchestrator::new(profile, cache, http), cache_path))
}

async fn run_serve() -> anyhow::Result<()> {
    let (orch, cache_path) = build_orchestrator()?;
    let state = AppState::new(orch);
    tracing::info!(version = VERSION, cache = %cache_path.display(), "tractd starting");
    server::serve(state).await
}

async fn run_status() -> anyhow::Result<()> {
    let resp = client::send(Method::Status).await?;
    if !resp.ok {
        let err = resp.error.unwrap_or_else(|| {
            tract_proto::ProtoError::new(
                tract_proto::ErrorCode::Internal,
                "unknown error from daemon",
            )
        });
        eprintln!(
            "daemon returned error: {} {}",
            code_str(err.code),
            err.message
        );
        std::process::exit(1);
    }
    match resp.result {
        Some(ResultBody::Status {
            version,
            proto,
            uptime_secs,
            cache_entries,
        }) => {
            println!("tractd {version} (proto v{proto})");
            println!("  uptime: {uptime_secs}s");
            println!("  cache:  {cache_entries} entries");
            Ok(())
        }
        other => {
            eprintln!("unexpected response shape: {other:?}");
            std::process::exit(1);
        }
    }
}

async fn run_fetch(url: String, force_refresh: bool, json: bool) -> anyhow::Result<()> {
    let (orch, _cache_path) = build_orchestrator()?;
    let opts = FetchOptions {
        force_refresh,
        return_details: false,
    };
    match orch.fetch(&url, opts).await {
        Ok(result) => {
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                if let Some(title) = result.title.as_deref() {
                    eprintln!("# {title}");
                }
                eprintln!("--- url:        {}", result.final_url);
                eprintln!("--- from_cache: {}", result.from_cache);
                eprintln!("--- trace_id:   {}", result.trace_id);
                eprintln!("---");
                println!("{}", result.markdown);
            }
            Ok(())
        }
        Err(err) => {
            eprintln!("fetch failed: {} {}", code_str(err.code), err.message);
            std::process::exit(1);
        }
    }
}

fn code_str(c: tract_proto::ErrorCode) -> &'static str {
    use tract_proto::ErrorCode::*;
    match c {
        InvalidRequest => "invalid_request",
        UnknownMethod => "unknown_method",
        InvalidUrl => "invalid_url",
        FetchFailed => "fetch_failed",
        ExtractFailed => "extract_failed",
        Timeout => "timeout",
        Internal => "internal",
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .try_init();
}
