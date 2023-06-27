/*
 * Command line arguments
 */
use clap::Parser;

/// lightweight animated backgrounds.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Start with desktop background
    #[arg(short, long)]
    pub background: Option<bool>,

    /// Start without configuration window
    #[arg(short, long)]
    pub no_window: bool,

    /// Start with system tray menu
    #[arg(short, long)]
    pub tray: Option<bool>,

    /// Set a specific scene
    #[arg(short, long)]
    pub scene: Option<String>,

    #[arg(long, hide = true)]
    pub system_startup: bool,
}
