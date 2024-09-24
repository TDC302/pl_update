
use crate::parse_manifest;
use crate::OptionArgs;

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



pub fn pl_repair(options: OptionArgs) -> std::io::Result<()> {


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

    if !options.target.is_empty() {
        match set_current_dir(options.target.clone()) {
            Ok(()) => (),
            Err(err) => {
                pl_update_fatal_error!("Could not find playlist directory: {}", err);
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
            
            //song_name = file_name.split_at(file_name.rfind('[').unwrap()).0.trim();
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
                pl_update_fatal_error!("The directory does not have an existing manifest, either run pl-update with the INIT command, or rename an old manifest to 'playlist.manifest'");

            } else {
                pl_update_fatal_error!("{e}");
            }
        }
    }

    let (_, playlist_title, playlist_url) = parse_manifest(File::open(old_playlist_filename)?)?;

    let mut manifest = File::create_new("playlist.manifest")?;

    manifest.write(format!("playlist_title={}{SEP_CHAR}url={}\n", playlist_title, playlist_url).as_bytes())?;

    for i in 0..song_names.len() {
        manifest.write(format!("title={}{SEP_CHAR}id={}{SEP_CHAR}url=\n", song_names.get(i).unwrap(), song_ids.get(i).unwrap()).as_bytes())?;
    }



    //let rejected_songs_str = rejected_songs.join("|");




    //pl_update_vprintln!("Reject string is: {:?}", rejected_songs_str);
    
    


    //let output_reader: BufReader<ChildStdout>; 
    //let err_reader: BufReader<ChildStderr>; 


    // let mut output_args = vec![playlist_url, "--extract-audio".to_owned(),
    //     "--audio-format=mp3".to_owned(), "--embed-thumbnail".to_owned(), "--add-metadata".to_owned()];

    // if options.verbose {
    //     output_args.push("--verbose".to_owned());
    // } else if options.quiet {
    //     output_args.push("--quiet".to_owned());
    // }



    // let filter = format!("id !~= {}", rejected_songs_str);
    // if !rejected_songs_str.is_empty() {
    //     output_args.push("--match-filter".to_owned());
    //     output_args.push(filter);

    // }

    // let ffmpeg_command;

    // if !options.ffmpeg_command.is_none() {
    //     output_args.push("--ffmpeg-location".to_owned());
    //     output_args.push(options.ffmpeg_command.to_owned().unwrap());
    //     ffmpeg_command = options.ffmpeg_command.unwrap();
    // } else {
    //     ffmpeg_command = "ffmpeg".to_owned();
    // }

    // find_ffmpeg(options.verbose, &ffmpeg_command)?;


    // let command_name = options.yt_dl_command.unwrap_or("yt-dlp".to_owned());
    // find_yt_dl(options.verbose, &command_name)?;

    
    
    // let mut ytdl_download = Command::new(command_name)
    //     .args(output_args)
    //     .stderr(Stdio::piped())
    //     .stdout(Stdio::piped()) //Set ytdl to have a piped output so we can use its output later.
    //     .spawn()?; //Run YTDL as a child process.





    // output_reader = BufReader::new(ytdl_download.stdout.take().unwrap()); //Get a handle to ytdl's output.
    // err_reader = BufReader::new(ytdl_download.stderr.take().unwrap()); //Get a handle to ytdl's output.


    


    // let skipped_songs ;
    // let songs_in_playlist;
    // let unavailable_songs;


    // let (tx, rx) = mpsc::channel();

    // let procid = ytdl_download.id();

    // let tx1 = tx.clone();
    // let child_err_handler: JoinHandle<Result<i32, String>> = 
    //                 thread::spawn(move || parse_ytdl_stderr(err_reader, tx1, procid));

    // let child_ytdl_handler: JoinHandle<(i32,i32)> =
    //                 thread::spawn(move ||  parse_ytdl_stdout(output_reader, tx, procid));


    



    // while !child_err_handler.is_finished() || !child_ytdl_handler.is_finished() {
        
        
    //     print!("{}", rx.recv().unwrap_or("".to_string()));


    // }


    // (songs_in_playlist, skipped_songs) = child_ytdl_handler.join().expect("Unknown error");

    // unavailable_songs = match child_err_handler.join().expect("Unknown error") {
    //     Ok(val) => val,
    //     Err(e) => {pl_update_fatal_error!("{}", e);}
    // };
    






    // ytdl_download.kill()?; //Clean up :)

    
    // if songs_in_playlist < 0 {

    //     pl_update_println!("Rejected {} songs. Detected {} songs in directory.", skipped_songs, songs_in_dir);
    //     pl_update_error!("Could not determine number of songs in playlist!");

    // } else {
    //     pl_update_println!("Successfully downloaded {} of {} songs in playlist. Detected {} songs in directory.", songs_in_playlist - unavailable_songs - skipped_songs, songs_in_playlist, songs_in_dir);

    // }

    
    // if unavailable_songs > 0 {
    //     pl_update_warn!("{} songs were unavailable for download.", unavailable_songs);
    // }







    Ok(())
}