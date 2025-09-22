use std::{fs::read_dir, io::{self, Error}};
use colored::Colorize;
use widestring::{Utf16String, WideUtfString};

use crate::{app::YOUTUBE_ID_LEN, debug_print, playlist_settings::PLAYLIST_SETTINGS_NAME_COMPAT, stdout_print, string_parsing::StringExts, warn_print, FILE_EXT};

use super::App;

use crate::Song;


impl App {

    pub(super) fn fetch_manifest_local(&self) -> Result<Vec<Song>, Error> {

    
        macro_rules! cnd_print_debug {
            ($($x:tt)*) => {
                if self.args.verbose {
                    debug_print!(
                        $(
                            $x
                        )*
                    );
                }
            };
        }

        macro_rules! cnd_print_stdout {
            ($($x:tt)*) => {
                if !self.args.quiet {
                    stdout_print!(
                    $(
                        $x
                    )*
                );
                }
            };
        }


        cnd_print_stdout!("Fetching contents of playlist \"{}\" from disk...", self.settings.playlist_name);

        
        // This list contains all files in the target directory.
        let directory_entry = read_dir(".")?.collect::<Result<Vec<_>, io::Error>>().unwrap();

        // at least one file in the directory should be the playlist settings, so this is the best guess for alloc size
        let mut songs = Vec::with_capacity(directory_entry.len()-1);
    
        for file_entry in directory_entry {

            let file_name = WideUtfString::from_os_string(file_entry.file_name()).unwrap();

            

            if Self::file_is_manifest(&file_name) { // skip manifests without warning
                continue;
            }

            if !file_name.ends_with(FILE_EXT.into()) {
                warn_print!("Loose file \"{}\" in directory.",  file_name);
                continue;
            }


            if let Some((song_name, remainder)) = file_name.rsplit_once("[".into()) {// If it has an opening bracket
                
                if remainder.len() != FILE_EXT.len() + YOUTUBE_ID_LEN + 2 {
                    warn_print!("Non yt-dl file \"{}\" in directory.", file_name);
                    continue;
                }

                let song = Song::new(song_name[0..song_name.len()-1].to_owned(),remainder[1..remainder.len() - FILE_EXT.len() - 1].to_owned(), None);
                cnd_print_debug!("Found {:?}", song);
                songs.push(song);

                
            } else {
                warn_print!("Non yt-dl file \"{}\" in directory.", file_name);
                continue;

            } 

        

        }
    
        Ok(songs)
    
    }

   
    pub(super) fn file_is_manifest(filename: &Utf16String) -> bool {
        if filename == &Utf16String::from_str(PLAYLIST_SETTINGS_NAME_COMPAT) || filename.starts_with(".".into()){
            true
        } else {
            false
        }
    }


}