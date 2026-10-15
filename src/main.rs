mod config;
mod game;
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

    let name = match cli.command {
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
            return render::run_demo(&scenario, seed, speed, &config.classes);
        }
        Command::Stage { .. } => "stage",
        Command::Ask { .. } => "ask",
        Command::Autostart => "autostart",
        Command::OpenAsk => "open-ask",
        Command::ToggleStage => "toggle-stage",
        Command::Doctor => "doctor",
    };
    eprintln!("{name}: not implemented");
    Ok(())
}
