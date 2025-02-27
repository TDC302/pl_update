extern crate chrono;
extern crate os_info;


mod string_parsing;
mod error;
mod playlist_settings;
mod app;


use core::str;
use std::fmt::Debug;
use app::{App, Args};
use clap::Parser;





#[macro_export]
macro_rules! pl_update_ok_exit {
    () => {
        println!("\n\n[pl-update] Operation Completed.");
        std::process::exit(0);
    };
}




#[derive(Debug, Clone)]
struct Song {
    title: String,
    id: String,
    url: Option<String>,
}

impl PartialEq for Song {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Song {
    fn new(title: String, id: String, url: Option<String>) -> Self {
        Song {title, id, url}
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

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let mut app = App::new(Args::parse());
    app.run()

}











