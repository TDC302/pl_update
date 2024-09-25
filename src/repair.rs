
use crate::parse_manifest;

use crate::Args;
use crate::SEP_CHAR;
use colored::Colorize;
//mod main;

use crate::pl_update_warn;
use crate::pl_update_fatal_error;

use chrono::Local;
use std::env::set_current_dir;
use std::fs;
use std::fs::read_dir;
use std::fs::File;
use std::io;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Write;
use std::time::SystemTime;



pub(crate) fn pl_repair(options: Args, playlist_name: Option<String>) -> std::io::Result<()> {


    const FILE_EXT: &str = ".mp3";

  


    // macro_rules! pl_update_println {
    //     ($($x:expr),*) => {
    //         if !options.quiet {
    //             println!("[pl-update] {}",
    //             format! (
    //                     $(
    //                         $x,
    //                     )*
    //                 )
    //             )
    //         }
    //     };
    // }

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
        match set_current_dir(playlist_name.clone().unwrap()) {
            Ok(()) => (),
            Err(err) => {
                pl_update_fatal_error!(err.kind(), "Could not find playlist directory: {}", err);
            }

        }
    } 

    const YOUTUBE_ID_LEN: usize = 11; //The length of a youtube ID, these get placed at the end of every file name so...
                                    //they need to be removed before being set to YTDL.

    //This list contains all files in the target directory.
    let directory_entry = read_dir(".")?.collect::<Result<Vec<_>, io::Error>>().unwrap();

    let mut song_ids: Vec<String> = Vec::new();
    let mut song_names = Vec::new();

    //let mut songs_in_dir = 0;

    
   
    for file_entry in directory_entry {

        let file_name = file_entry.file_name().into_string().expect("File name was not string!");
        let song_name: &str;
        let remainder: &str;



        if !file_name.ends_with(FILE_EXT){
            pl_update_warn!("Loose file \"{}\" in directory.",  file_name);
            continue;
        }


        if file_name.contains('[') {// If it has an opening bracket
            
            
            (song_name, remainder) = file_name.split_at(file_name.rfind('[').unwrap());
            
            

            if remainder.len() != FILE_EXT.len() + YOUTUBE_ID_LEN + 2 {
                pl_update_warn!("Non yt-dl file \"{}\" in directory.", file_name);
                continue;
            }

            
        } else {
            pl_update_warn!("Non yt-dl file \"{}\" in directory.", file_name);
            continue;

        } 

        //songs_in_dir += 1;

       

        song_ids.push((&remainder[1..remainder.len() - FILE_EXT.len() - 1]).to_string());
        song_names.push((&song_name[0..song_name.len()-1]).to_string());

    }

    
    pl_update_vprintln!("Song names: {:?}", song_names);
    pl_update_vprintln!("Song Ids: {:?}", song_ids);


    let time: chrono::DateTime<Local> =  SystemTime::now().into();
    let old_playlist_filename = format!("playlist-{}.manifest", time.format("%Y-%m-%dT%H%M%S%.f"));

    match fs::rename("playlist.manifest", old_playlist_filename.clone()) {
        Ok(()) => (),
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                pl_update_fatal_error!(ErrorKind::NotFound, "The directory does not have an existing manifest, either run pl-update with the INIT command, or rename an old manifest to 'playlist.manifest'");

            } else {
                pl_update_fatal_error!(e.kind(), "Could not rename playlist.manifest: {}", e);
            }
        }
    }

    let (_, playlist_title, playlist_url) = parse_manifest(File::open(old_playlist_filename)?)?;

    let mut manifest = File::create_new("playlist.manifest")?;

    manifest.write(format!("playlist_title={}{SEP_CHAR}url={}\n", playlist_title, playlist_url).as_bytes())?;

    for i in 0..song_names.len() {
        manifest.write(format!("title={}{SEP_CHAR}id={}{SEP_CHAR}url=\n", song_names.get(i).unwrap(), song_ids.get(i).unwrap()).as_bytes())?;
    }




    Ok(())
}