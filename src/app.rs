use std::{env::{self}, fs::OpenOptions, io::{self, ErrorKind}, path::Path, process::Command, str, time::SystemTime};
use colored::Colorize;
use chrono::Local;
use commands::Commands;
pub(crate) use args::Args;

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




pub struct App {
    args: Args,
    settings: Option<PlaylistSettings>

}

impl App {



    pub fn new(args: Args) -> Self {
        Self { args, settings: None }
    }

  
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let time: chrono::DateTime<Local> = SystemTime::now().into();
        let info = os_info::get();
    
        let ver = env!("CARGO_PKG_VERSION");

        #[cfg(debug_assertions)] 
        env::set_var("RUST_BACKTRACE", "1");


        println!("pl-update version {}", ver);

        #[cfg(debug_assertions)] 
        println!("Debugging build");

        
        if info.architecture().is_some() {
            println!("Running on {} {} for {}", info.os_type(), info.version(), info.architecture().unwrap());
        } else {
            println!("Running on {} version {}", info.os_type(), info.version());
        }

        println!("Started at system time {}\n", time.format("%+"));

        if self.args.verbose {
            debug_print!("Args: {:?}", self.args);
        }

        if self.args.quiet {
            self.args.suppress_interactive = true;
        }

        let command = self.args.command.clone();

        let ret: Result<(), Box<dyn std::error::Error>> = match command {
            Commands::Init { playlist_url } => self.pl_init(playlist_url).map_err(|e| e.into()),
            Commands::Push { playlist_name, device_id } => self.pl_push(playlist_name, device_id).map_err(|e| e.into()),
            Commands::Repair { playlist_name } => self.pl_repair(playlist_name).map_err(|e| e.into()),
            Commands::Update { playlist_name } => self.update(playlist_name).map_err(|e| e.into())
    
        };
    
        let new_time: chrono::DateTime<Local> = SystemTime::now().into();
    
        let delta = new_time - time;
    
    
        match ret {
            Ok(_) => {
    
                stdout_print!("Operation completed in {}m {}s", delta.num_minutes(), delta.num_seconds());
                Ok(())
            },
            Err(e) => {
                eprintln!("\n{} {}\n", "FATAL ERROR:".red().bold(), e);
                Err(e)
            }
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




fn find_ffmpeg(&self) -> Result<(), io::Error> {
    let ffmpeg_command = self.args.ffmpeg_location.clone();
    let ffmpeg_check: Result<std::process::Output, io::Error> = Command::new(&ffmpeg_command).arg("-version").output();
        
        if ffmpeg_check.is_ok() {
            let out = &ffmpeg_check.unwrap().stdout;
            let out_data = str::from_utf8(out).unwrap().split(" ").collect::<Vec<_>>();
            let ver = out_data.get(2).unwrap();
            if self.args.verbose {
                debug_print!("Found {} version {}", ffmpeg_command, ver);
            }
            return Ok(());
            
        } 

    fatal_error!(ErrorKind::NotFound, "FFMPEG could not be found. Check that it is in the system path or current directory and is accessible.");


}


fn fetch_settings<P: AsRef<Path>>(&mut self, path: P) -> Result<(), io::Error> {
    let file = OpenOptions::new().read(true).open(path)?;
    self.settings = Some(PlaylistSettings::from_file(file)?);

    Ok(())
}




}