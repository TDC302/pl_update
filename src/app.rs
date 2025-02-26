use std::{env, time::SystemTime};
use colored::Colorize;
use chrono::Local;
use commands::Commands;
use args::Args;

mod args;
mod commands;

mod update;
mod repair;
mod push;
mod init;

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



pub struct App {
    args: Args
}

impl App {



    pub fn new(args: Args) -> Self {
        Self { args }
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
    
                println!("[pl-update] Operation completed in {}m {}s", delta.num_minutes(), delta.num_seconds());
                Ok(())
            },
            Err(e) => {
                eprintln!("\n{} {}\n", "FATAL ERROR:".red().bold(), e);
                Err(e)
            }
        }
    

    }

   
}