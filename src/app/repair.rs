
use crate::error::RepairError;

use crate::playlist_settings::PlaylistSettings;
use colored::Colorize;

use crate::warn_print;

use std::env::set_current_dir;
use std::fs;
use std::io;
use std::io::stdout;
use std::io::ErrorKind;
use std::io::Write;

use super::App;

impl App {


    pub(super) fn repair(&mut self, playlist_name: Option<String>) -> Result<(), RepairError> {

        if playlist_name.is_some() {
            set_current_dir(playlist_name.clone().unwrap())?;
        } 

        match fs::OpenOptions::new().read(true).open("playlist-settings.json") {
            Ok(f) => {
                if let Err(_) = PlaylistSettings::from_file(f) {
                    Self::manual_settings(&playlist_name)?;
                }
            },
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    if self.args.suppress_interactive {
                        return Err(RepairError::PlaylistUninitialized);
                    } else {
                        Self::manual_settings(&playlist_name)?;
                    }
                } else {
                    return Err(e.into());
                }
            }
        }


        Ok(())
    }


    /// Interactively prompt the user to create a new playlist settings file
    pub(super) fn manual_settings(playlist_name: &Option<String>) -> Result<(), std::io::Error> {
        warn_print!("A playlist settings file could not be found for the current directory, or it is corrupt and could not be read.");
        warn_print!("Attempting to download this playlist will fail.");
        print!("\nWould you like to create a new settings file now? (y/n): ");
        stdout().flush()?;
        loop {

            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer)?;

            if buffer.trim().eq_ignore_ascii_case("y") {
                print!("Enter playlist url: ");
                stdout().flush()?;
                buffer.clear();
                io::stdin().read_line(&mut buffer)?;
                let url = buffer.trim().to_string();

                let name = 
                if playlist_name.is_some() {
                    playlist_name.clone().unwrap()
                } else {
                    print!("Enter playlist name: ");
                    stdout().flush()?;
                    buffer.clear();
                    io::stdin().read_line(&mut buffer)?;
                    buffer.trim().to_string()
                };

                PlaylistSettings::new(url, name).write_to_disk("playlist-settings.json")?;

                println!("\nSuccessfully created new playlist settings.");
                break;

            } else if buffer.trim().eq_ignore_ascii_case("n") {
                break;
            } else {
                print!("(y/n): ");
                buffer.clear();
                continue;
            }
            
        }

        Ok(())
    }
}