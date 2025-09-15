
use crate::error::Error;

use crate::playlist_settings;
use crate::playlist_settings::PlaylistSettings;
use colored::Colorize;

use crate::warn_print;

use std::fs::create_dir;
use std::fs::remove_file;
use std::io;
use std::io::stdout;
use std::io::Write;

use super::App;

impl App {


    pub(super) fn repair(&mut self, playlist_name: Option<String>) -> Result<(), Error> {

        match PlaylistSettings::try_open() {
            Ok(s) => {
                if s.format_is_depreciated() {
                    Self::update_settings_format(&s)?;
                } else {
                    println!("No issues found.");
                }
            }
            Err(Error::PlaylistUninitialized) => {
                if self.args.suppress_interactive {
                        return Err(Error::PlaylistUninitialized);
                    } else {
                        Self::manual_settings(&playlist_name)?;
                    }
            }
            Err(e) => return Err(e)
        } 


        Ok(())
    }


    pub(super) fn update_settings_format(playlist_settings: &PlaylistSettings) -> Result<(), std::io::Error> {
        println!("Updating playlist format...");
                
        create_dir(".playlist")?;
        playlist_settings.write_to_disk()?;
        remove_file(playlist_settings::PLAYLIST_SETTINGS_NAME_COMPAT)?;

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

                create_dir(".playlist")?;
                PlaylistSettings::new(url, name).write_to_disk()?;

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