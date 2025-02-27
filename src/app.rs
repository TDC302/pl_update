use std::io::ErrorKind;
use std::{env::set_current_dir, fs::OpenOptions, str};
use clap::Parser;
use colored::Colorize;
use commands::Commands;
pub(crate) use args::Args;
use crate::error::Error;

use crate::playlist_settings::PlaylistSettings;

mod args;
mod commands;

mod update;
mod repair;
mod push;
mod init;
mod manifest;
mod download;

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
const YOUTUBE_ID_LEN: usize = 11;

/// The filename to use for playlist settings files.
const PLAYLIST_SETTINGS_NAME: &str = "playlist-settings.json";


pub struct App {
    args: Args,
    settings: PlaylistSettings
    
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
            
            if playlist_name_.is_some() {
                set_current_dir(playlist_name_.unwrap())?;
            }

            let file = match OpenOptions::new().read(true).open(PLAYLIST_SETTINGS_NAME) {
                Ok(val) => val,
                Err( e) =>  {
                    if e.kind() == ErrorKind::NotFound {
                        return Err(Error::PlaylistUninitialized);
                    } else {
                        return Err(Error::FileOpenError(PLAYLIST_SETTINGS_NAME.to_string(), e))
                    }
                }
            };

            settings =  PlaylistSettings::from_file(file)?;
        }

        let mut app = Self {args, settings};

        match command {
            Commands::Init { playlist_url } => app.init(playlist_url).map_err(|e| e.into()),
            Commands::Push { playlist_name, device_id } => app.push(playlist_name, device_id).map_err(|e| e.into()),
            Commands::Repair { playlist_name } => app.repair(playlist_name).map_err(|e| e.into()),
            Commands::Update { .. } => app.update().map_err(|e| e.into())
        }
    
      
    }


    fn find_yt_dl(&self) -> Result<(), std::io::Error> {

        let ytdl_command = self.args.yt_dl_location.clone();
        let ytdl_check: Result<std::process::Output, std::io::Error> = std::process::Command::new(&ytdl_command).arg("--version").output();
        
        if ytdl_check.is_ok() {
            let out = &ytdl_check.unwrap().stdout;
            let ver = std::str::from_utf8(out).unwrap().trim();
            if self.args.verbose {
                debug_print!("Found {} version {}", ytdl_command, ver);
            }
            return Ok(());
            
        } else {
            let e = ytdl_check.unwrap_err();
            fatal_error!(e.kind(), "YT-DL could not be launched. Check that it is in the system path or current directory and is accessible. \nReason: {}", e);
        
        }
    }




}