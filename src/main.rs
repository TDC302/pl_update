extern crate chrono;
extern crate os_info;


mod string_parsing;
mod error;
mod playlist_settings;
mod app;


use core::str;
use std::{env, fmt::Debug, time::SystemTime};
use app::App;
use chrono::Local;
use colored::Colorize;
use widestring::Utf16String;
use crate::error::Error;



#[derive(Debug, Clone)]
struct Song {
    title: Utf16String,
    id: Utf16String,
    url: Option<String>,
}

impl PartialEq for Song {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Song {
    fn new(title: Utf16String, id: Utf16String, url: Option<String>) -> Self {
        Song {title, id, url}
    }

    fn new_str(title: String, id: String, url: Option<String>) -> Self {
        Self { title: title.into(), id: id.into(), url }
    }

    fn into_filename(&self, file_ext: String) -> String {
        format!("{} [{}].{}", self.title, self.id, file_ext)
    }

    fn url(&self) -> Option<String> {
        self.url.clone()
    }
}


const SEP_CHAR: char = '\x06'; 

const FILE_EXT: &str = ".mp3";

fn main() -> Result<(), Error> {
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


    let ret = App::run();

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











