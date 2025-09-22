use std::env::set_current_dir;
use clap::Parser;
use colored::Colorize;
use commands::Commands;
pub(crate) use args::Args;
use crate::downloader::Downloader;
use crate::error::Error;

use crate::playlist_settings::PlaylistSettings;

mod args;
mod commands;

mod update;
mod repair;
mod push;
mod init;
mod manifest;

#[macro_export]
macro_rules! stdout_print {
    ($($x:expr),*) => {
        println!("[pl-update] {}",
        format! (
                $(
                    $x,
                )*
            )
        )
    };
}

#[macro_export]
macro_rules! debug_print {
    ($($x:expr),*) => {
        println!("{} [pl-update] {}", "DEBUG:".blue(),
        format! (
            $(
                $x,
            )*
        )
        )
        
    };
}

#[macro_export]
macro_rules! error_print {
    ($($x:expr),*) => {
        eprintln!("{} [pl-update] {}", "ERROR:".red().bold(),
        format! (
            $(
                $x,
            )*
        )
        )
    };
}

#[macro_export]
macro_rules! warn_print {
    ($($x:expr),*) => {
        eprintln!("{} [pl-update] {}", "WARNING:".yellow(),
        format! (
            $(
                $x,
            )*
        )
        )
    };
}

#[macro_export]
macro_rules! fatal_error {
    ($ekind:expr, $($e:expr),*) => {

        let emsg = format!(
            $(
                $e,
            )*
        );
        
        let kind = $ekind;

        #[cfg(debug_assertions)]
        return Err(std::io::Error::new(kind, format!("{}\nFile: {}, Line: {}", emsg, file!(), line!())));

        #[cfg(not(debug_assertions))]
        return Err(std::io::Error::new(kind, emsg));
        
    
    };
    ($err:tt) => {

        #[cfg(debug_assertions)]
        let emsg = format!("{}\nFile: {}, Line: {}", $err.to_string(), file!(), line!());
        
        #[cfg(not(debug_assertions))]
        let emsg = $err.to_string();

        return Err(std::io::Error::new($err.kind(), emsg));
        
    };

}

/// The length of a youtube ID.
pub(crate) const YOUTUBE_ID_LEN: usize = 11;

pub struct App {
    args: Args,
    settings: PlaylistSettings,
    downloader: Downloader
    
}

impl App {

    pub fn run() -> Result<(), Error> {
        let mut args = Args::parse();

        if args.verbose {
            debug_print!("Args: {:?}", args);
        }

        if args.quiet {
            args.suppress_interactive = true;
        }

        let command = args.command.clone();

        let settings;
        if let Commands::Init { ref playlist_url } = command {
            settings = PlaylistSettings::new(playlist_url.to_string(), String::new());
        } else {

            let playlist_name_;
            if let Commands::Push { ref playlist_name,..} = command {
                playlist_name_ = playlist_name.clone();
            } else if let Commands::Update {ref playlist_name} = command {
                playlist_name_ = playlist_name.clone();
            } else if let Commands::Repair {ref playlist_name} = command {
                playlist_name_ = playlist_name.clone();
            } else {
                unreachable!();
            }
            
            
            if let Some(mut dir) = playlist_name_ {
                // there is some weird issue with clap where it doesn't properly parse paths that end with slash
                if dir.ends_with('\\') { dir.truncate(dir.len()-1); }

                set_current_dir(dir)?; // there should be a better error message here.
            }
            settings =  PlaylistSettings::try_open()?;
            if settings.format_is_depreciated() {
                warn_print!("Playlist settings uses old format, consider running 'pl-update repair'");
            }
        }
        let downloader = Downloader::new(args.yt_dl_location.clone(), args.yt_dl_args.clone())?;

        if args.verbose {
            debug_print!("Found {} version {}", args.yt_dl_location, downloader.version())
        }

        let mut app = Self {args, settings, downloader};

        match command {
            Commands::Init { playlist_url } => app.init(playlist_url).map_err(|e| e.into()),
            Commands::Push { playlist_name, device_id } => app.push(playlist_name, device_id).map_err(|e| e.into()),
            Commands::Repair { playlist_name } => app.repair(playlist_name).map_err(|e| e.into()),
            Commands::Update { .. } => app.update().map_err(|e| e.into())
        }
    
      
    }


}