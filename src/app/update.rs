use chrono::Local;
use colored::Colorize;
use std::{env::set_current_dir, fs::{self, remove_file, File, OpenOptions}, io::ErrorKind, time::SystemTime};

use crate::{error::UpdateError, error_print as pl_update_error, warn_print};

use super::App;



impl App {


    pub(super) fn update(&mut self, playlist_name: Option<String>) -> Result<(), UpdateError>{

        macro_rules! pl_update_vprintln {
            ($($x:expr),*) => {
                if self.args.verbose {
                    println!("{} [pl-update] {}", "DEBUG:".blue(),
                    format! (
                        $(
                            $x,
                        )*
                    )

                    )
                }
            };
        }

        macro_rules! pl_update_println {
            ($($x:expr),*) => {
                if !&self.args.quiet {
                    println!("[pl-update] {}",
                    format! (
                            $(
                                $x,
                            )*
                        )
                    )
                }
            };
        }
        self.find_ffmpeg()?;
        self.find_yt_dl()?;
    
        if playlist_name.is_some() {
            match set_current_dir(playlist_name.clone().unwrap()) {
                Ok(()) => (),
                Err(err) => {
                    return Err(UpdateError::DirectoryOpenError(playlist_name.unwrap(), err));
                }

            }
        } 

        let time: chrono::DateTime<Local> =  SystemTime::now().into();
        

        let current_manifest = match OpenOptions::new().read(true).write(true).create(true).open("playlist.manifest") {
            Ok(val) => val,
            Err(err) => {
                return Err(UpdateError::FileOpenError("playlist manifest".to_owned(), err));
            }
        }; 
        
        let current_songs = Self::parse_manifest(current_manifest)?;
        self.fetch_settings("playlist-settings.json")?;
        let settings = self.settings.as_ref().unwrap();
        let playlist_name = &settings.playlist_name;
        let playlist_url = &settings.playlist_url;
        let cleanup_count = settings.backup_manifest_count.unwrap_or(7);

        pl_update_println!("Found playlist: \"{}\"", playlist_name);

        pl_update_println!("Updating manifest...");

        let new_manifest = match File::create_new("playlist-new.manifest") {
            Ok(val) => val,
            Err(e) => {
                if e.kind() == ErrorKind::AlreadyExists {
                    warn_print!("playlist-new.manifest already exists. This likely indicates a download in progress failed. This file will be overrwritten.");
                    OpenOptions::new().read(true).write(true).open("playlist-new.manifest")?
                } else {
                    return Err(UpdateError::FileCreationError("playlist-new.manifest".to_owned(), e));
                }
            }
        };

        self.update_manifest(new_manifest, playlist_name, playlist_url)?;

        let new_songs = Self::parse_manifest(File::open("playlist-new.manifest")?)?;
        

        let removed_songs: Vec<_> = 
        current_songs.clone().into_iter().filter(|old_song|
        
            !new_songs.contains(old_song)

        ).collect();

        let added_songs: Vec<_> = new_songs.into_iter().filter(|new_song|

            !current_songs.contains(new_song)
        
        ).collect();

        pl_update_vprintln!("Detected {} items to download: {:?}", added_songs.len(), added_songs);

        let removed_filenames: Vec<_> = removed_songs.into_iter().map(|f| f.into_filename("mp3".to_owned())).collect();
        let added_urls: Vec<_> = added_songs.into_iter().map(|u| u.url().unwrap()).collect();

        pl_update_vprintln!("Detected {} items to remove: {:?}", removed_filenames.len(), removed_filenames);


        
        if added_urls.len() > 0 {
            pl_update_println!("Downloading {} new items...", added_urls.len());
            self.download(added_urls)?;
        } else {
            pl_update_println!("No items to download.");
        }
        

        if removed_filenames.len() > 0 {
            pl_update_println!("Deleting {} removed items...", removed_filenames.len()); 

            for filename in removed_filenames {
                pl_update_println!("Deleting file {}.", &filename);

                if let Err(e) = remove_file(&filename) {
                    pl_update_error!("Error while deleting file \"{}\"\t {}", &filename, e);
                }
            }
        }  else {
            pl_update_println!("No items to remove.")
        }



        let old_playlist_filename = format!("playlist-{}.manifest", time.format("%Y-%m-%dT%H%M%S"));
        fs::rename("playlist.manifest", old_playlist_filename)?;
        fs::rename("playlist-new.manifest", "playlist.manifest")?;

        self.cleanup_old_manifests( cleanup_count)?;


        Ok(())
    }
}
