use crate::find_ffmpeg;
use crate::find_yt_dl;
use crate::OptionArgs;

use colored::Colorize;
//mod main;

use crate::pl_update_warn;
use crate::pl_update_error;
use crate::pl_update_fatal_error;

use std::fs::read_dir;
use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Error;
use std::process::ChildStderr;
use std::process::ChildStdout;
use std::process::Command;
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;



pub fn pl_get(options: OptionArgs) -> std::io::Result<()> {


    const FILE_EXT: &str = ".mp3";

    let mut bytes_read;


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

    if options.url.is_empty() {
        pl_update_fatal_error!("GET command requires a playlist url.");
    }


    let playlist_url = options.url;

    const YOUTUBE_ID_LEN: usize = 11; //The length of a youtube ID, these get placed at the end of every file name so...
                                    //they need to be removed before being set to YTDL.

    //This list contains all files in the target directory.
    let directory_entry = read_dir(".")?.collect::<Result<Vec<_>, io::Error>>().unwrap();

    let mut rejected_songs: Vec<String> = Vec::new();


    let mut songs_in_dir = 0;

    
   
    for file_entry in directory_entry {

        let file_name = file_entry.file_name().into_string().expect("File name was not string!");
        //let song_name: &str;
        let remainder: &str;



        if !file_name.ends_with(FILE_EXT){
            pl_update_warn!("Loose file \"{}\" in directory.",  file_name);
            continue;
        }


        if file_name.contains('[') {// If it has an opening bracket
            
            //song_name = file_name.split_at(file_name.rfind('[').unwrap()).0.trim();
            remainder = file_name.split_at(file_name.rfind('[').unwrap()).1;
            
            

            if remainder.len() != FILE_EXT.len() + YOUTUBE_ID_LEN + 2 {
                pl_update_warn!("Non yt-dl file \"{}\" in directory.", file_name);
                continue;
            }

            
        } else if file_name.contains("-") { 
            
            //song_name = file_name.split_at(file_name.rfind('-').unwrap()).0;
            remainder = file_name.split_at(file_name.rfind('-').unwrap()).1;

            if remainder.len() != FILE_EXT.len() + YOUTUBE_ID_LEN + 1 {
                pl_update_warn!("Non yt-dl file \"{}\" in directory.", file_name);
                continue;
            }

     
        }  else {
            pl_update_warn!("Non yt-dl file \"{}\" in directory.", file_name);
            continue;

        } 

        songs_in_dir += 1;

       

        rejected_songs.push(regex::escape(&remainder[1..remainder.len() - FILE_EXT.len() - 1]));


    }

    
    
    
    

    let rejected_songs_str = rejected_songs.join("|");




    pl_update_vprintln!("Reject string is: {:?}", rejected_songs_str);
    
    


    let mut output_reader: BufReader<ChildStdout>; 
    let mut err_reader: BufReader<ChildStderr>; 


    let mut output_args = vec![playlist_url, "--extract-audio".to_owned(),
        "--audio-format=mp3".to_owned(), "--embed-thumbnail".to_owned(), "--add-metadata".to_owned()];

    if options.verbose {
        output_args.push("--verbose".to_owned());
    } else if options.quiet {
        output_args.push("--quiet".to_owned());
    }

    if !options.yt_dl_args.is_empty() {
        let mut user_args = options.yt_dl_args.clone();
        output_args.append(&mut user_args);
    }

    let filter = format!("id !~= {}", rejected_songs_str);
    if !rejected_songs_str.is_empty() {
        output_args.push("--match-filter".to_owned());
        output_args.push(filter);

    }

    let ffmpeg_command;

    if !options.ffmpeg_command.is_none() {
        output_args.push("--ffmpeg-location".to_owned());
        output_args.push(options.ffmpeg_command.to_owned().unwrap());
        ffmpeg_command = options.ffmpeg_command.unwrap();
    } else {
        ffmpeg_command = "ffmpeg".to_owned();
    }

    find_ffmpeg(options.verbose, &ffmpeg_command)?;


    let command_name = options.yt_dl_command.unwrap_or("yt-dlp".to_owned());
    find_yt_dl(options.verbose, &command_name)?;

    
    
    let mut ytdl_download = Command::new(command_name)
        .args(output_args)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped()) //Set ytdl to have a piped output so we can use its output later.
        .spawn()?; //Run YTDL as a child process.





    output_reader = BufReader::new(ytdl_download.stdout.take().unwrap()); //Get a handle to ytdl's output.
    err_reader = BufReader::new(ytdl_download.stderr.take().unwrap()); //Get a handle to ytdl's output.


    

    bytes_read = 1; //Make sure not to exit loop immediately!

    let mut skipped_songs = 0;
    let mut songs_in_playlist = -1;

    let mut unavailable_songs = 0;


    let (tx, rx) = mpsc::channel();


    let tx1 = tx.clone();
    let child_err_handler: JoinHandle<Result<i32, String>> = thread::spawn(move || {

        

        let mut err_str: String = String::new();
        let mut err_bytes_read = 1;


        while err_bytes_read > 0 {


            
            let err_val = err_reader.read_line(&mut err_str);

            let out_str;

            if err_val.is_err() {
                pl_update_error!("Read from YT-DL STDERR buffer failed with error: \"{}\"!", err_val.unwrap_err());
                err_str.clear();
                continue;

            } else {
               err_bytes_read = err_val.unwrap();
            }



            if err_str.starts_with("[debug] ") {
                out_str = format!("{} [yt-dl] {}", "DEBUG:".blue(), err_str.split_off("[debug] ".len()));

            } else if err_str.starts_with("WARNING:") {
                out_str = format!("{}{}", "WARNING:".yellow(), err_str.split_off("WARNING:".len()));

            } else if err_str.starts_with("ERROR:") {
                if err_str.contains("Video unavailable.") {
                    err_str.pop();
                    err_str.push_str(".\n"); //Add a period because I can.
                    unavailable_songs += 1;
                } else if err_str.contains("Unsupported URL:") {
                    return Err(err_str.split_off("ERROR:".len()));
                }

                out_str = format!("{}{}", "ERROR:".red().bold(), err_str.split_off("ERROR:".len()));


            } else {
                out_str = format!("{}", err_str); 
                
            }
            
            tx1.send(out_str).unwrap();

            err_str.clear();

        }

        Ok(unavailable_songs)
    });


    let child_ytdl_handler: JoinHandle<(i32,i32)> = thread::spawn( move || {


        let mut buffer_str: String = String::new();

        while bytes_read > 0 { //When the output reader returns 0 bytes read, we know ytdl is done.

            
            

            
            bytes_read = output_reader.read_line(&mut buffer_str).unwrap(); //Read a line from YTDL's output.

            let out_str;

            if songs_in_playlist == -1 && buffer_str.starts_with("[youtube:tab] Playlist ") {
                
                songs_in_playlist = buffer_str.trim().rsplit_once(" ").unwrap().1.parse().unwrap();
                
            } 
            
            //YTDL will print the whole reject pattern (which contains every song in the folder) every time it finds a match.
            //This makes console output unreadable, so this code cuts it off.
            if buffer_str.contains("not pass filter") {
                out_str = format!("{}, skipping..\n", buffer_str.split_inclusive("not pass filter").next().unwrap());
                skipped_songs += 1;

            } else {
                out_str = format!("{}", buffer_str);

            }
            
            tx.send(out_str).unwrap();
            buffer_str.clear();


        }
        (songs_in_playlist, skipped_songs)


    });



    while !child_err_handler.is_finished() || !child_ytdl_handler.is_finished() {
        
        
        print!("{}", rx.recv().unwrap_or("".to_string()));


    }


    (songs_in_playlist, skipped_songs) = child_ytdl_handler.join().expect("Unknown error");
    
    
    unavailable_songs = match child_err_handler.join().expect("Unknown error") {
        Ok(val) => val,
        Err(e) => {pl_update_fatal_error!("{}", e);}
    };
    






    ytdl_download.kill()?; //Clean up :)

    
    if songs_in_playlist < 0 {

        pl_update_println!("Rejected {} songs. Detected {} songs in directory.", skipped_songs, songs_in_dir);
        pl_update_error!("Could not determine number of songs in playlist!");

    } else {
        pl_update_println!("Successfully downloaded {} of {} songs in playlist. Detected {} songs in directory.", songs_in_playlist - unavailable_songs - skipped_songs, songs_in_playlist, songs_in_dir);

    }

    
    if unavailable_songs > 0 {
        pl_update_error!("{} songs were unavailable for download.", unavailable_songs);
    }







    Ok(())
}