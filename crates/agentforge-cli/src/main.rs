use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process;
use uuid::Uuid;

use agentforge_db::{
    agent_repo::AgentRepo, create_pool, eval_repo::EvalRepo, scenario_repo::ScenarioRepo,
    trace_repo::TraceRepo,
};
use agentforge_gatekeeper::{GateStatus, Gatekeeper, GatekeeperConfig};
use agentforge_parser::{parse_agent_file, to_agent_version, validate_agent_file};
use agentforge_runner::{AgentRunner, AnthropicClient, OpenAiClient, RunnerConfig};
use agentforge_scenarios::{generate_scenarios, ScenarioGeneratorConfig};
use agentforge_scorer::{score_run, ScorerConfig};

#[derive(Parser)]
#[command(
    name = "agentforge",
    about = "AI agent optimization and evaluation platform",
    version,
    author
)]
struct Cli {
    /// Increase verbosity (-v debug, -vv trace)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run evaluation on an agent file
    Run {
        /// Path to the agent YAML/JSON file
        #[arg(short, long)]
        agent: PathBuf,

        /// Number of scenarios to generate
        #[arg(short, long, default_value = "100")]
        scenarios: u32,

        /// Concurrency for parallel scenario execution
        #[arg(short, long, default_value = "10")]
        concurrency: u32,

        /// Random seed for reproducibility
        #[arg(long, default_value = "42")]
        seed: u64,
    },

    /// Compare two agent versions
    Diff {
        /// First agent version ID (UUID)
        v1: Uuid,
        /// Second agent version ID (UUID)
        v2: Uuid,
    },

    /// Promote a run's agent version to champion
    Promote {
        /// Run ID to promote
        run_id: Uuid,
    },

    /// Show scores for a run
    Scores {
        /// Run ID to display scores for
        #[arg(long)]
        run: Uuid,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Load .env
    let _ = dotenvy::dotenv();

    // Init tracing based on verbosity
    let level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level)),
        )
        .init();

    let exit_code = match run_command(cli.command).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Error: {e:#}");
            2
        }
    };

    process::exit(exit_code);
}

/// Returns 0 for pass, 1 for gate failure, 2 for error.
async fn run_command(command: Commands) -> Result<i32> {
    match command {
        Commands::Run {
            agent,
            scenarios,
            concurrency,
            seed,
        } => cmd_run(agent, scenarios, concurrency, seed).await,
        Commands::Diff { v1, v2 } => cmd_diff(v1, v2).await,
        Commands::Promote { run_id } => cmd_promote(run_id).await,
        Commands::Scores { run } => cmd_scores(run).await,
    }
}

async fn cmd_run(
    agent_path: PathBuf,
    scenario_count: u32,
    concurrency: u32,
    seed: u64,
) -> Result<i32> {
    let content = std::fs::read_to_string(&agent_path)
        .with_context(|| format!("Failed to read agent file: {}", agent_path.display()))?;

    // Parse
    let parsed = parse_agent_file(&content).with_context(|| "Failed to parse agent file")?;

    // Validate
    let validation = validate_agent_file(&parsed.agent);
    for err in &validation.errors {
        eprintln!("[ERROR] {}", err.message);
    }
    for warn in &validation.warnings {
        eprintln!("[WARN]  {}", warn.message);
    }
    if !validation.errors.is_empty() {
        eprintln!(
            "Validation failed with {} error(s)",
            validation.errors.len()
        );
        return Ok(2);
    }

    let agent_file = parsed.agent.clone();
    let format = parsed.format.clone();
    let sha = parsed.sha.clone();
    println!(
        "Agent: {} v{} (format: {}, sha: {})",
        agent_file.name,
        agent_file.version,
        format,
        &sha[..12]
    );

    // DB (optional — skip if no DATABASE_URL)
    let db_opt = if let Ok(url) = std::env::var("DATABASE_URL") {
        let pool = create_pool(&url).await.ok();
        if let Some(ref pool) = pool {
            let _ = agentforge_db::run_migrations(pool).await;
        }
        pool
    } else {
        None
    };

    // Store agent version in DB if available
    let agent_id = if let Some(ref db) = db_opt {
        let repo = AgentRepo::new(db.clone());
        let agent_version = to_agent_version(parsed.clone());
        match repo.find_by_sha(&sha).await? {
            Some(existing) => existing.id,
            None => repo.insert(&agent_version).await?.id,
        }
    } else {
        Uuid::new_v4()
    };

    // Generate scenarios
    println!("Generating {} scenarios...", scenario_count);
    let scorer_config = build_scorer_config();
    let scenarios = generate_scenarios(
        &agent_file,
        &ScenarioGeneratorConfig {
            total_count: scenario_count,
            agent_id,
            llm_base_url: Some(scorer_config.judge_base_url.clone()),
            llm_api_key: if scorer_config.judge_api_key.is_empty() {
                None
            } else {
                Some(scorer_config.judge_api_key.clone())
            },
            llm_model: Some(scorer_config.judge_model.clone()),
            ..Default::default()
        },
    )
    .await
    .with_context(|| "Scenario generation failed")?;
    println!("Generated {} scenarios.", scenarios.len());

    // Build LLM client
    let llm_client = build_llm_client()?;

    // Run agent
    println!(
        "Running agent across {} scenarios (concurrency: {})...",
        scenarios.len(),
        concurrency
    );
    let runner = AgentRunner::new(
        llm_client,
        RunnerConfig {
            concurrency: concurrency as usize,
            ..Default::default()
        },
    );
    let run_result = runner
        .run(
            &agent_file,
            scenarios.clone(),
            Some(std::sync::Arc::new(move |done: u32, total: u32| {
                if done.is_multiple_of(10) || done == total {
                    print!("\r  Progress: {}/{} scenarios", done, total);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
            })),
        )
        .await;
    let mut traces = run_result.traces;
    println!();

    // Score
    let run_id = Uuid::new_v4();
    let scorecard = score_run(&mut traces, &scenarios, &agent_file, run_id, &scorer_config)
        .await
        .with_context(|| "Scoring failed")?;

    // Print results
    print_scorecard(&scorecard);

    // Persist to DB if available
    if let Some(ref db) = db_opt {
        let eval_repo = EvalRepo::new(db.clone());
        let scenario_repo = ScenarioRepo::new(db.clone());
        let trace_repo = TraceRepo::new(db.clone());

        let new_run = agentforge_core::EvalRun {
            id: Uuid::new_v4(),
            agent_id,
            scenario_set_id: None,
            status: agentforge_core::EvalRunStatus::Pending,
            scenario_count: scenarios.len() as u32,
            completed_count: 0,
            error_count: 0,
            aggregate_score: None,
            pass_rate: None,
            scores: None,
            failure_clusters: None,
            seed: seed as u32,
            concurrency,
            error_message: None,
            started_at: None,
            completed_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        let eval_run = eval_repo.insert(&new_run).await?;
        scenario_repo.insert_batch(&scenarios).await?;
        for trace in &traces {
            let _ = trace_repo.insert(trace).await;
        }
        eval_repo
            .save_scores(
                eval_run.id,
                &scorecard.dimension_scores,
                scorecard.aggregate_score,
                scorecard.pass_rate,
                &scorecard.failure_clusters,
            )
            .await?;
        eval_repo
            .update_status(eval_run.id, &agentforge_core::EvalRunStatus::Complete)
            .await?;
        println!("\nRun ID: {}", eval_run.id);
    }

    // Exit code: 0 = pass threshold met, 1 = failed
    let pass_threshold = agent_file
        .eval_hints
        .as_ref()
        .and_then(|h| h.pass_threshold)
        .unwrap_or(0.85);

    if scorecard.aggregate_score >= pass_threshold {
        Ok(0)
    } else {
        eprintln!(
            "\nFailed: aggregate score {:.3} < threshold {:.3}",
            scorecard.aggregate_score, pass_threshold
        );
        Ok(1)
    }
}

async fn cmd_diff(v1: Uuid, v2: Uuid) -> Result<i32> {
    let db = require_db().await?;
    let repo = AgentRepo::new(db);

    let ver1 = repo
        .find_by_id(v1)
        .await
        .map_err(|_| anyhow::anyhow!("Version {v1} not found"))?;
    let ver2 = repo
        .find_by_id(v2)
        .await
        .map_err(|_| anyhow::anyhow!("Version {v2} not found"))?;

    println!(
        "Diff: {} v{} → {} v{}",
        ver1.name, ver1.version, ver2.name, ver2.version
    );
    println!("SHA: {} → {}", &ver1.sha[..12], &ver2.sha[..12]);

    let prompt1 = ver1.file_content.system_prompt.as_str();
    let prompt2 = ver2.file_content.system_prompt.as_str();

    if prompt1 != prompt2 {
        println!(
            "\nSystem prompt changed ({} → {} chars)",
            prompt1.len(),
            prompt2.len()
        );
    } else {
        println!("\nSystem prompt: unchanged");
    }

    Ok(0)
}

async fn cmd_promote(run_id: Uuid) -> Result<i32> {
    let db = require_db().await?;
    let eval_repo = EvalRepo::new(db.clone());
    let agent_repo = AgentRepo::new(db.clone());
    let trace_repo = TraceRepo::new(db.clone());

    let run = eval_repo
        .find_by_id(run_id)
        .await
        .map_err(|_| anyhow::anyhow!("Run {run_id} not found"))?;

    if run.status != agentforge_core::EvalRunStatus::Complete {
        anyhow::bail!("Run {run_id} is not complete (status: {:?})", run.status);
    }

    let challenger_scorecard = run
        .to_scorecard()
        .ok_or_else(|| anyhow::anyhow!("Run {run_id} has no scores"))?;

    let challenger_traces = trace_repo.list_by_run(run_id).await?;

    // Find current champion by agent name — fetch agent first to get name
    let challenger_agent = agent_repo
        .find_by_id(run.agent_id)
        .await
        .map_err(|_| anyhow::anyhow!("Agent not found"))?;
    let champion_versions = agent_repo.list_by_name(&challenger_agent.name).await?;
    let champion = champion_versions.iter().find(|v| v.is_champion);
    let champion_scorecard = if let Some(champ) = champion {
        let runs = eval_repo.list_by_agent(champ.id, 1).await?;
        runs.into_iter().next().and_then(|r| r.to_scorecard())
    } else {
        None
    };

    let champion_passing = if let Some(champ) = champion {
        let runs = eval_repo.list_by_agent(champ.id, 1).await?;
        if let Some(champ_run) = runs.into_iter().next() {
            trace_repo.list_passing_scenario_ids(champ_run.id).await?
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let seed_scores = vec![challenger_scorecard.aggregate_score; 3];

    let gk = Gatekeeper::new(GatekeeperConfig::default());
    let decision = gk.evaluate(
        run_id,
        run.agent_id,
        champion_scorecard.as_ref(),
        &challenger_scorecard,
        &champion_passing,
        &challenger_traces,
        &seed_scores,
    )?;

    println!(
        "\nPromotion Decision: {}",
        if decision.approved {
            "APPROVED ✅"
        } else {
            "DENIED ❌"
        }
    );
    println!();
    for gate in &decision.gates {
        let sym = match gate.status {
            GateStatus::Pass => "✅",
            GateStatus::Fail => "❌",
            GateStatus::Waived => "⏭",
        };
        println!("{} {} — {}", sym, gate.gate, gate.message);
    }
    println!("\n{}", decision.changelog);

    if decision.approved {
        agent_repo
            .set_champion(challenger_agent.id, &challenger_agent.name)
            .await?;
        agent_repo
            .update_changelog(challenger_agent.id, &decision.changelog)
            .await?;
        println!("\nAgent promoted to champion.");
        Ok(0)
    } else {
        Ok(1)
    }
}

async fn cmd_scores(run_id: Uuid) -> Result<i32> {
    let db = require_db().await?;
    let eval_repo = EvalRepo::new(db);
    let run = eval_repo
        .find_by_id(run_id)
        .await
        .map_err(|_| anyhow::anyhow!("Run {run_id} not found"))?;

    let scorecard = run
        .to_scorecard()
        .ok_or_else(|| anyhow::anyhow!("Run {run_id} has no scores yet"))?;

    print_scorecard(&scorecard);
    Ok(0)
}

fn print_scorecard(sc: &agentforge_core::Scorecard) {
    println!("\n╔══════════════════════════════════════╗");
    println!("║          AgentForge Scorecard         ║");
    println!("╠══════════════════════════════════════╣");
    println!("║  Agent:      {} v{}", sc.agent_name, sc.agent_version);
    println!("║  Run ID:     {}", sc.run_id);
    println!("╠══════════════════════════════════════╣");
    println!("║  Aggregate:  {:.3}", sc.aggregate_score);
    println!(
        "║  Pass Rate:  {:.1}% ({}/{} scenarios)",
        sc.pass_rate * 100.0,
        sc.passed,
        sc.total_scenarios
    );
    println!("╠══════════════════════════════════════╣");
    println!("║  Dimension Scores:                   ║");
    println!(
        "║    Task Completion:    {:.3}",
        sc.dimension_scores.task_completion
    );
    println!(
        "║    Tool Selection:     {:.3}",
        sc.dimension_scores.tool_selection
    );
    println!(
        "║    Arg Correctness:    {:.3}",
        sc.dimension_scores.argument_correctness
    );
    println!(
        "║    Schema Compliance:  {:.3}",
        sc.dimension_scores.schema_compliance
    );
    println!(
        "║    Instr. Adherence:   {:.3}",
        sc.dimension_scores.instruction_adherence
    );
    println!(
        "║    Path Efficiency:    {:.3}",
        sc.dimension_scores.path_efficiency
    );
    if !sc.failure_clusters.is_empty() {
        println!("╠══════════════════════════════════════╣");
        println!("║  Failure Clusters:");
        for cluster in &sc.failure_clusters {
            println!(
                "║    {:?}: {} ({:.0}%)",
                cluster.cluster,
                cluster.count,
                cluster.percentage * 100.0
            );
        }
    }
    println!("╠══════════════════════════════════════╣");
    println!(
        "║  Duration: {}s  Tokens: {}in/{}out",
        sc.duration_seconds, sc.total_input_tokens, sc.total_output_tokens
    );
    println!("╚══════════════════════════════════════╝");
}

fn build_scorer_config() -> ScorerConfig {
    ScorerConfig {
        judge_model: std::env::var("AGENTFORGE_JUDGE_MODEL")
            .unwrap_or_else(|_| "gpt-4o".to_string()),
        judge_base_url: std::env::var("AGENTFORGE_JUDGE_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
        judge_api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
        ..Default::default()
    }
}

fn build_llm_client() -> Result<std::sync::Arc<dyn agentforge_runner::LlmClient>> {
    let provider =
        std::env::var("AGENTFORGE_JUDGE_PROVIDER").unwrap_or_else(|_| "openai".to_string());
    match provider.as_str() {
        "anthropic" => Ok(std::sync::Arc::new(
            AnthropicClient::from_env()
                .ok_or_else(|| anyhow::anyhow!("ANTHROPIC_API_KEY must be set"))?,
        )
            as std::sync::Arc<dyn agentforge_runner::LlmClient>),
        _ => Ok(std::sync::Arc::new(
            OpenAiClient::from_env()
                .ok_or_else(|| anyhow::anyhow!("OPENAI_API_KEY must be set"))?,
        )
            as std::sync::Arc<dyn agentforge_runner::LlmClient>),
    }
}

async fn require_db() -> Result<agentforge_db::PgPool> {
    let url =
        std::env::var("DATABASE_URL").context("DATABASE_URL must be set for database commands")?;
    create_pool(&url)
        .await
        .context("Failed to connect to database")
}
