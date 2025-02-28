use colored::Colorize;
use clap::Parser;
use super::commands::Commands;
use crate::warn_print;



/// Playlist manager for yt-dlp
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, author)]
#[command(propagate_version = true)]
pub(crate) struct Args {

    /// Print extra debugging information
    #[arg(short, long, default_value_t = false)]
    pub(crate) verbose: bool,

    /// Suppress output. Also disables interactive promps
    #[arg(short, long, default_value_t = false)]
    pub(crate) quiet: bool,

    /// Disable interactive prompts
    #[arg(long)]
    pub(crate) suppress_interactive: bool,


    /// Args to pass to yt-dlp
    #[arg(long)]
    pub(crate) yt_dl_args: Vec<String>,


    /// The location of yt-dlp
    #[arg(long, default_value_t = {"yt-dlp".to_string()})] 
    pub(crate) yt_dl_location: String,

    /// Args provided to ffmpeg to run on every file after it is downloaded
    /// not implemented
    #[arg(long)]
    pub(crate) postproccessor_args: Vec<String>,


    /// The number of threads to use
    #[arg(short, long, default_value_t = {
        let cpu_core_count = match std::thread::available_parallelism() {
            Ok(val) => val,
            Err(e) => {
                warn_print!("Core count unknown, defaulting to single core mode.");
                return 1.to_string();
            }
        }.get();

        if cpu_core_count <= 0 {
            panic!("System has no cpu cores.");
        }

        if cpu_core_count >= 16 {
            cpu_core_count / 4
        } else if cpu_core_count >= 8 {
            cpu_core_count / 3
        } else if cpu_core_count >= 4 {
            cpu_core_count / 2
        } else  {
            1
        }
    
    })]
    pub(crate) threads: usize,


    #[command(subcommand)]
    pub(crate) command: Commands


}
