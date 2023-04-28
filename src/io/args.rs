/*
 * Command line arguments
 */
use clap::Parser;

/// lightweight animated backgrounds.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Start the desktop background
    #[arg(short, long)]
    pub background: Option<bool>,

    /// Start the configurator window
    #[arg(short, long)]
    pub window: Option<bool>,

    /// Start the system tray menu
    #[arg(short, long)]
    pub tray: Option<bool>,

    /// Default scene
    #[arg(short, long)]
    pub scene: Option<String>,

    /// Scene directory (by default, [shaderbg path]/scenes/)
    #[arg(short = 'd', long)]
    pub scene_dir: Option<std::path::PathBuf>,
}
