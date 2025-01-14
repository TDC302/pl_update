
use crate::cleanup_old_manifests;
use crate::error::RepairError;
use crate::file_is_manifest;

use crate::playlist_settings::PlaylistSettings;
use crate::string_parsing::StringExts;
use crate::Args;
use crate::FILE_EXT;
use crate::SEP_CHAR;
use colored::Colorize;
use widestring::WideUtfString;

use crate::pl_update_warn;

use chrono::Local;
use std::env::set_current_dir;
use std::fs;
use std::fs::read_dir;
use std::fs::File;
use std::io;
use std::io::stdout;
use std::io::ErrorKind;
use std::io::Write;
use std::time::SystemTime;



pub(crate) fn pl_repair(options: Args, playlist_name: Option<String>) -> Result<(), RepairError> {

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

    if playlist_name.is_some() {
        set_current_dir(playlist_name.clone().unwrap())?;
    } 

    /// The length of a youtube ID.
    const YOUTUBE_ID_LEN: usize = 11;

    match fs::OpenOptions::new().read(true).open("playlist-settings.json") {
        Ok(f) => {
            if let Err(_) = PlaylistSettings::from_file(f) {
                manual_settings(&playlist_name)?;
            }
        },
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                if options.suppress_interactive {
                    return Err(RepairError::PlaylistUninitialized);
                } else {
                    manual_settings(&playlist_name)?;
                }
            } else {
                return Err(e.into());
            }
        }
    }



    //This list contains all files in the target directory.
    let directory_entry = read_dir(".")?.collect::<Result<Vec<_>, io::Error>>().unwrap();

    let mut song_ids = Vec::new();
    let mut song_names = Vec::new();
    
   
    for file_entry in directory_entry {

        let file_name = WideUtfString::from_os_string(file_entry.file_name())?;

        

        if file_is_manifest(&file_name) { // skip manifests without warning
            continue;
        }

        if !file_name.ends_with(FILE_EXT.into()) {
            pl_update_warn!("Loose file \"{}\" in directory.",  file_name);
            continue;
        }


        if let Some((song_name, remainder)) = file_name.rsplit_once("[".into()) {// If it has an opening bracket
            
            if remainder.len() != FILE_EXT.len() + YOUTUBE_ID_LEN + 2 {
                pl_update_warn!("Non yt-dl file \"{}\" in directory.", file_name);
                continue;
            }

            song_ids.push(remainder[1..remainder.len() - FILE_EXT.len() - 1].to_owned());
            song_names.push(song_name[0..song_name.len()-1].to_owned());
            
        } else {
            pl_update_warn!("Non yt-dl file \"{}\" in directory.", file_name);
            continue;

        } 

       

    }

    
    pl_update_vprintln!("Song names: {:?}", song_names);
    pl_update_vprintln!("Song Ids: {:?}", song_ids);


    let time: chrono::DateTime<Local> =  SystemTime::now().into();
    let old_playlist_filename = format!("playlist-{}.manifest", time.format("%Y-%m-%dT%H%M%S-r"));

    match fs::rename("playlist.manifest", old_playlist_filename.clone()) {
        Ok(()) => (),
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                ()
            } else {
                return Err(e.into());
            }
        }
    }

 

    

    let mut manifest = File::create_new("playlist.manifest")?;

    for i in 0..song_names.len() {
        manifest.write(format!("title={}{SEP_CHAR}id={}{SEP_CHAR}url=\n", song_names.get(i).unwrap(), song_ids.get(i).unwrap()).as_bytes())?;
    }


    cleanup_old_manifests(&options, 7)?;


    Ok(())
}

fn manual_settings(playlist_name: &Option<String>) -> Result<(), std::io::Error> {
    pl_update_warn!("A playlist settings file could not be found for the current directory, or it is corrupt and could not be read.");
    pl_update_warn!("Attempting to download this playlist will fail.");
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