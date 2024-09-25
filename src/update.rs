use chrono::Local;
use colored::Colorize;
use std::{env::set_current_dir, fs::{self, remove_file, File, OpenOptions}, io::Error, time::SystemTime};

use crate::{download, parse_manifest, pl_update_fatal_error, update_manifest, Args};




pub(crate) fn pl_update(options: Args, playlist_name: Option<String>) -> Result<(), Error>{

    macro_rules! pl_update_vprintln {
        ($($x:expr),*) => {
            if options.verbose {
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
            if !options.quiet {
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
    
  
    if playlist_name.is_some() {
        match set_current_dir(playlist_name.unwrap().clone()) {
            Ok(()) => (),
            Err(err) => {
                pl_update_fatal_error!(err.kind(), "Could not find playlist directory: {}", err);
            }

        }
    } 

    let time: chrono::DateTime<Local> =  SystemTime::now().into();
    let old_playlist_filename = format!("playlist-{}.manifest", time.format("%Y-%m-%dT%H%M%S%.f"));

    match fs::rename("playlist.manifest", old_playlist_filename.clone()) {
        Ok(()) => {},
        Err(e) => {pl_update_fatal_error!(e.kind(), "Could not rename old playlist manifest: {}", e);}
    };


    let old_manifest = match OpenOptions::new().read(true).write(true).create(true).open(old_playlist_filename) {
        Ok(val) => val,
        Err(err) => {
            pl_update_fatal_error!(err.kind(), "Could not open playlist manifest: {}", err);
        }
    }; 
     
    let (old_songs, playlist_name, playlist_url) = parse_manifest(old_manifest)?;


    pl_update_println!("Found playlist: \"{}\"", playlist_name);

    pl_update_println!("Updating manifest...");

    let new_manifest = File::create_new("playlist.manifest")?;

    update_manifest(new_manifest, playlist_name, &playlist_url, &options)?;

    let (new_songs, _, _) = parse_manifest(File::open("playlist.manifest")?)?;
    

    let removed_songs: Vec<_> = 
    old_songs.clone().into_iter().filter(|old_song|
    
        !new_songs.contains(old_song)

    ).collect();

    let added_songs: Vec<_> = new_songs.into_iter().filter(|new_song|

        !old_songs.contains(new_song)
    
    ).collect();

    pl_update_vprintln!("Files to download: {:?}", added_songs);

    let removed_filenames: Vec<_> = removed_songs.into_iter().map(|f| f.into_filename("mp3".to_owned())).collect();
    let added_urls: Vec<_> = added_songs.into_iter().map(|u| u.url().unwrap()).collect();

    pl_update_vprintln!("Files to remove: {:?}", removed_filenames);


    
    if added_urls.len() > 0 {
        pl_update_println!("Downloading...");
        download(added_urls, &options)?;
    } else {
        pl_update_println!("No files to download.");
    }
    

    if removed_filenames.len() > 0 {
        pl_update_println!("Removing old files..."); 
        for filename in removed_filenames {
            remove_file(filename)?;
        }
    }  else {
        pl_update_println!("No files to remove.")
    }

    

    Ok(())
}