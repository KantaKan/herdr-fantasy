mod config;
mod game;
mod herdr;
mod logging;
mod palette;
mod render;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "herdr-fantasy",
    version,
    about = "Your coding agents as an RPG party"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Main pane: battle scene, party, and HUD (long-running TUI)
    Stage {
        /// Run a scripted fake party; works outside herdr
        #[arg(long)]
        demo: bool,
        /// Demo scenario: party, boss, town, stress
        #[arg(long, default_value = "party")]
        scenario: String,
        /// RNG seed for reproducible demos
        #[arg(long)]
        seed: Option<u64>,
        /// Demo playback speed multiplier
        #[arg(long, default_value_t = 1.0)]
        speed: f32,
        /// Print average CPU and frame time after 60 s (demo only)
        #[arg(long)]
        bench: bool,
    },
    /// JRPG popup for blocked agents (short-lived)
    Ask {
        /// Show the popup with fake text
        #[arg(long)]
        demo: bool,
    },
    /// Startup / workspace.created: open the stage if missing
    Autostart,
    /// Action: open the ask popup for the most recently blocked hero
    OpenAsk,
    /// Action: show/hide the stage
    ToggleStage,
    /// Print detected terminal graphics, socket, repo, and config
    Doctor,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let _log_guard = logging::init();
    let (config, warnings) = config::load();
    for w in &warnings {
        tracing::warn!("config: {w}");
    }

    let result = match cli.command {
        Command::Stage {
            demo: true,
            scenario,
            seed,
            speed,
            ..
        } => {
            let seed = seed.unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_nanos() as u64)
            });
            return render::run_demo(&scenario, seed, speed, &config);
        }
        Command::Stage { .. } => return render::run_live(&config),
        Command::Ask { demo } => return render::ask::run(&config, demo),
        // herdr runs these as hooks/actions: log failures, never fail the hook.
        Command::Autostart => herdr::actions::autostart(config.auto_open),
        Command::OpenAsk => herdr::actions::open_ask(),
        Command::ToggleStage => herdr::actions::toggle_stage(),
        Command::Doctor => return doctor(&config),
    };
    if let Err(e) = result {
        tracing::warn!("{e:#}");
        eprintln!("{e:#}");
    }
    Ok(())
}

fn doctor(config: &config::Config) -> anyhow::Result<()> {
    let sock = herdr::client::socket_path();
    println!("socket:  {}", sock.display());
    match herdr::client::Client::connect()
        .and_then(|mut c| c.request("agent.list", serde_json::json!({})))
    {
        Ok(r) => println!(
            "herdr:   ok, {} agents",
            r["agents"].as_array().map_or(0, Vec::len)
        ),
        Err(e) => println!("herdr:   not reachable ({e:#})"),
    }
    println!(
        "config:  {}",
        config::config_path().map_or("none".into(), |p| p.display().to_string())
    );
    println!("render:  {:?} (halfblock until M9)", config.render);
    Ok(())
}
