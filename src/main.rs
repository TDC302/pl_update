extern crate chrono;
extern crate os_info;


mod push;
mod init;
mod update;
mod repair;
mod string_parsing;
mod error;
mod playlist_settings;

use std::env::current_dir;
use std::io::{self, ErrorKind};
use core::{panic, str};
use std::fmt::Debug;
use std::path::Path;
use std::{env, thread};
use std::fs::{remove_file, File, OpenOptions};
use std::io::{BufRead, BufReader, Error, Read, Write};
use std::process::{ChildStderr, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, SyncSender};
use std::thread::{sleep, JoinHandle};
use std::time::{Duration, SystemTime};
use chrono::Local;
use clap::{Parser, Subcommand};
use colored::Colorize;
use playlist_settings::PlaylistSettings;
use string_parsing::StringExts;
use widestring::Utf16String;


#[macro_export]
macro_rules! pl_update_warn {
    ($($x:expr),*) => {
        eprintln!("{} [pl-update] {}", "WARNING:".yellow(),
        format! (
            $(
                $x,
            )*
        )
        )
    };
}

#[macro_export]
macro_rules! pl_update_error {
    ($($x:expr),*) => {
        eprintln!("{} [pl-update] {}", "ERROR:".red().bold(),
        format! (
            $(
                $x,
            )*
        )
        )
    };
}

#[macro_export]
macro_rules! pl_update_fatal_error {
    ($ekind:expr, $($e:expr),*) => {

        let emsg = format!(
            $(
                $e,
            )*
        );
        
        let kind = $ekind;

        #[cfg(debug_assertions)]
        return Err(Error::new(kind, format!("{}\nFile: {}, Line: {}", emsg, file!(), line!())));

        #[cfg(not(debug_assertions))]
        return Err(Error::new(kind, emsg));
        
    
    };
    ($err:tt) => {

        #[cfg(debug_assertions)]
        let emsg = format!("{}\nFile: {}, Line: {}", $err.to_string(), file!(), line!());
        
        #[cfg(not(debug_assertions))]
        let emsg = $err.to_string();

        return Err(Error::new($err.kind(), emsg));
        
    };

}


#[macro_export]
macro_rules! pl_update_ok_exit {
    () => {
        println!("\n\n[pl-update] Operation Completed.");
        std::process::exit(0);
    };
}
/// Playlist manager for yt-dlp
#[derive(Parser, Debug)]
#[command(version, about, long_about = None, author)]
#[command(propagate_version = true)]
struct Args {

    /// Print extra debugging information
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Suppress output. Also disables interactive promps
    #[arg(short, long, default_value_t = false)]
    quiet: bool,

    /// Disable interactive prompts
    #[arg(long)]
    suppress_interactive: bool,


    /// Args to pass to yt-dlp
    #[arg(long)]
    yt_dl_args: Vec<String>,


    /// The location of yt-dlp
    #[arg(long, default_value_t = {"yt-dlp".to_string()})] 
    yt_dl_location: String,

    /// Args provided to ffmpeg to run on every file after it is downloaded
    #[arg(long)]
    postproccessor_args: Vec<String>,



    /// The location of yt-dlp
    #[arg(long)] 
    ffmpeg_location: Option<String>,


    /// The number of threads to use
    #[arg(short, long, default_value_t = {
        let cpu_core_count = match std::thread::available_parallelism() {
            Ok(val) => val,
            Err(e) => {
                pl_update_warn!("Core count unknown, defaulting to single core mode.");
                return 1.to_string();
            }
        }.get();

        if cpu_core_count <= 0 {
            panic!("System has no cpu cores.");
        }

        if cpu_core_count >= 24 {
            cpu_core_count / 4
        } else if cpu_core_count >= 12 {
            cpu_core_count / 3
        } else if cpu_core_count >= 4 {
            cpu_core_count / 2
        } else  {
            1
        }
    
    })]
    threads: usize,


    #[command(subcommand)]
    command: Commands


}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Creates a new directory for a playlist, fetches the playlist manifest
    /// and downloads all associated songs.
    Init { 
        /// The url of the playlist to be downloaded
        playlist_url: String 
    },
    /// Checks playlist for new or removed songs, and downloads/deletes files respectively. 
    /// Requires a valid manifest containing the playlist url.
    Update { 
        /// Optional. If provided the application will use this as the playlist directory.
        playlist_name: Option<String> 
    },
    /// Rebuilds the playlist manifest from the files in the directory. 
    /// Requires a playlist manifest containing at least the playlist url.
    Repair { 
        /// Optional. If provided the application will use this as the playlist directory.
        playlist_name: Option<String> },
    /// Will send the files in the playlist to a connected media device (Android phone, music player) etc..
    /// If device id is not specified, and there is more than one device connected will prompt
    /// user to select device.
    Push { 
        /// Optional. If provided the application will use this as the playlist directory.
        playlist_name: Option<String>,
        /// Optional. The device id to send to.
        device_id: Option<String> 
    }
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
    let time: chrono::DateTime<Local> = SystemTime::now().into();
    let info = os_info::get();

 

    let ver = env!("CARGO_PKG_VERSION");

    #[cfg(debug_assertions)] 
    env::set_var("RUST_BACKTRACE", "1");


    println!("pl-update version {}", ver);

    #[cfg(debug_assertions)] 
    println!("Debugging build");

    
    if info.architecture().is_some() {
        println!("Running on {} {} for {}", info.os_type(), info.version(), info.architecture().unwrap());
    } else {
        println!("Running on {} version {}", info.os_type(), info.version());
    }

    println!("Started at system time {}\n", time.format("%+"));

    
    let mut args = Args::parse();
    

    // macro_rules! pl_update_println {
    //     ($($x:expr),*) => {
    //         if !args.quiet {
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
            if args.verbose {
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

    
    pl_update_vprintln!("Args: {:?}", args);

    if args.quiet {
        args.suppress_interactive = true;
    }

    let command = args.command.clone();
 

    let ret: Result<(), Box<dyn std::error::Error>> = match command {
        Commands::Init { playlist_url } => init::pl_init(args, playlist_url).map_err(|e| e.into()),
        Commands::Push { playlist_name, device_id } => push::pl_push(args, playlist_name, device_id).map_err(|e| e.into()),
        Commands::Repair { playlist_name } => repair::pl_repair(args, playlist_name).map_err(|e| e.into()),
        Commands::Update { playlist_name } => update::pl_update(args, playlist_name).map_err(|e| e.into())

    };

    let new_time: chrono::DateTime<Local> = SystemTime::now().into();

    let delta = new_time - time;


    match ret {
        Ok(_) => {

            println!("[pl-update] Operation completed in {}m {}s", delta.num_minutes(), delta.num_seconds());


            Ok(())
        
        },
        Err(e) => {
            eprintln!("\n{} {}\n", "FATAL ERROR:".red().bold(), e);
            Err(e)
        }
    }


    
 

}


fn find_yt_dl(verbose: bool, ytdl_command: &String) -> Result<(), Error> {

   
    let ytdl_check: Result<std::process::Output, Error> = Command::new(ytdl_command.clone()).arg("--version").output();
    
    if ytdl_check.is_ok() {
        let out = &ytdl_check.unwrap().stdout;
        let ver = str::from_utf8(out).unwrap().trim();
        if verbose {
            println!("{} [pl-update] Found {} version {}", "DEBUG:".blue(), ytdl_command, ver);
        }
        return Ok(());
        
    } else {
        let e = ytdl_check.unwrap_err();
        pl_update_fatal_error!(e.kind(), "YT-DL could not be launched. Check that it is in the system path or current directory and is accessible. \nReason: {}", e);
    
    }
        
    

}


///
/// This function assumes that the current directory is the playlist directory and may have undefined behavior otherwise
fn cleanup_old_manifests(options: &Args, old_manifest_count: usize) -> Result<(), Error> {
    let files = std::fs::read_dir(current_dir()?)?.collect::<Result<Vec<_>, io::Error>>().unwrap();
    let mut manifests = Vec::new();
    for file in files {
        if file_is_manifest(&Utf16String::from_os_string(file.file_name()).unwrap()) {
            manifests.push(file);
        }
    }


    // nothing to do!
    if manifests.len() < old_manifest_count {
        return Ok(());
    }

    if !options.quiet {
        println!("[pl-update] Cleaning up...");
    }

    manifests.sort_by(|a, b | b.file_name().partial_cmp(&a.file_name()).unwrap());

    while manifests.len() >= 7 {
        let old_manifest = manifests.pop().expect("list should be at least 7 long");
        if options.verbose {
            println!("{} [pl-update] Deleted old manifest {}", "DEBUG:".blue(), old_manifest.file_name().to_str().unwrap());
        }
        remove_file(old_manifest.path())?;
    }

    return Ok(());

}

fn find_ffmpeg(verbose: bool, ffmpeg_command: &String) -> Result<(), Error> {



    let ffmpeg_check: Result<std::process::Output, Error> = Command::new(ffmpeg_command.clone()).arg("-version").output();
        
        if ffmpeg_check.is_ok() {
            let out = &ffmpeg_check.unwrap().stdout;
            let out_data = str::from_utf8(out).unwrap().split(" ").collect::<Vec<_>>();
            let ver = out_data.get(2).unwrap();
            if verbose {
                println!("{} [pl-update] Found {} version {}", "DEBUG:".blue(), ffmpeg_command, ver);
            }
            return Ok(());
            
        } 

    pl_update_fatal_error!(ErrorKind::NotFound, "FFMPEG could not be found. Check that it is in the system path or current directory and is accessible.");


}

fn parse_ytdl_stderr(mut std_err_reader: BufReader<ChildStderr>, tx: SyncSender<String>, procid: u32) -> Result<usize, String> {
    let mut err_str: String = String::new();
    let mut err_bytes_read = 1;
    let mut unavailable_songs: usize = 0;

        while err_bytes_read > 0 {


            
            let err_val = std_err_reader.read_line(&mut err_str);

            let out_str;

            if err_val.is_err() {
                pl_update_error!("Read from YT-DL STDERR buffer failed with error: \"{}\"!", err_val.unwrap_err());
                err_str.clear();
                continue;

            } else {
               err_bytes_read = err_val.unwrap();
            }



            if err_str.is_empty() {
                out_str = String::new();
            } else if err_str.starts_with("[debug] ") {
                out_str = format!("[thread {}] {} [yt-dl] {}", procid, "DEBUG:".blue(), err_str.split_off("[debug] ".len()));

            } else if err_str.starts_with("WARNING:") {
                out_str = format!("[thread {}] {}{}", procid, "WARNING:".yellow(), err_str.split_off("WARNING:".len()));

            } else if err_str.starts_with("ERROR:") {
                if err_str.contains("Video unavailable.") {
                    err_str.pop();
                    err_str.push_str(".\n"); //Add a period because I can.
                    unavailable_songs += 1;
                } else if err_str.contains("Unsupported URL:") {
                    return Err(err_str.split_off("ERROR:".len()));
                }

                out_str = format!("[thread {}] {}{}", procid, "ERROR:".red().bold(), err_str.split_off("ERROR:".len()));

            } else {
                out_str = format!("[thread {}] {}", procid, err_str); 
                
            }
            
            tx.send(out_str).unwrap();
            err_str.clear();

        }

        drop(tx);
    return Ok(unavailable_songs);


}

fn parse_ytdl_stdout(mut std_out_reader: BufReader<ChildStdout>, tx: SyncSender<String>, procid: u32) {
    

    let mut bytes_read = 1;
    let mut buffer_str: String = String::new();

    while bytes_read > 0 { //When the output reader returns 0 bytes read, we know ytdl is done.

        
        bytes_read = match std_out_reader.read_line(&mut buffer_str) //Read a line from YTDL's output.
        {
            Ok(val) => val,
            Err(e) => {
                if e.kind() == ErrorKind::InvalidData { // ignore invalid utf-8
                    continue;
                } else {
                    Err(e).unwrap() // all other errors, panic
                }
            }
        };
        if bytes_read > 0 {

            let out_str;

            out_str = format!("[thread {}] {}", procid, buffer_str);

            tx.send(out_str).unwrap();
            buffer_str.clear();

        }

    }

    drop(tx);


}

fn update_manifest(mut manifest: File, playlist_title: String, playlist_url: &String, options: &Args) -> Result<(), Error> {

    // manifest.write(format!("playlist_title={}{SEP_CHAR}url={}\n", playlist_title, playlist_url).as_bytes())?;
    
    let mut output_args = Vec::new();
    let command_name = &options.yt_dl_location;

    if options.verbose {
        output_args.push("--verbose".to_owned());
        output_args.push("--quiet".to_owned());
    }

    output_args.push("--restrict-filenames".to_owned());
    output_args.push("--windows-filenames".to_owned());
    output_args.push("--simulate".to_owned());
    output_args.push("--flat-playlist".to_owned());
    output_args.push("--lazy-playlist".to_owned());
    output_args.push(playlist_url.clone());
    
    output_args.push("--print".to_owned());

    
    output_args.push(format!("title=%(title)s{SEP_CHAR}id=%(id)s{SEP_CHAR}url=%(webpage_url)s"));


    if !options.quiet {
        println!("Fetching contents of playlist \"{playlist_title}\"");
    } 


    if options.verbose {
        println!("Running {} with arguments {:?}", command_name, output_args);
    }
  

    let mut ytdl_process = Command::new(&command_name)
    .args(&output_args)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;


    let err_reader = BufReader::new(ytdl_process.stderr.take().unwrap());
    let mut out_reader = BufReader::new(ytdl_process.stdout.take().unwrap());
    let (tx, rx) = mpsc::sync_channel(2);
    let procid = ytdl_process.id();
    let ytdl_err_handler = thread::spawn(move || parse_ytdl_stderr(err_reader, tx, procid));

    let ytdl_out_handler: JoinHandle<Result<(),Error>> = thread::spawn(move || {
        let buf = &mut vec![];
        out_reader.read_to_end(buf)?;
        let sliced_str = buf.utf8_chunks();

        let valid_string = sliced_str.fold(String::new(),|acc, chnk| acc + chnk.valid()); 
        manifest.write_all(valid_string.as_bytes())?;


    Ok(())

    });

    while !ytdl_err_handler.is_finished() || !ytdl_out_handler.is_finished() {
        print!("{}", rx.recv().unwrap_or("".to_string()));
    }

    

    Ok(())
}

fn parse_manifest(manifest: File) -> Result<Vec<Song>, Error> {

    //let mut playlist_title = "".to_string();
    //let mut playlist_url = "".to_string();


    let file_reader = BufReader::new(manifest);

    let entries = file_reader.lines();

    let mut songs = Vec::new();



    let mut line_num = 0;
    for entry in entries {
        line_num += 1;

        let entry_;


        if let Err(e) = entry {
            if e.kind() == ErrorKind::InvalidData {
                pl_update_warn!("Invalid data while parsing manifest line {line_num}!");
                continue;
            } else {
                return Err(e);
            }
        } else {
            entry_ = entry.unwrap();
        }

        // if line_num == 1 {
        //     let first: Vec<&str> = entry_.split(SEP_CHAR).collect();
            
        //     let name_ = first.get(0).expect("manifest should contain '\\x06'");
        //     let url_ = first.get(1).expect("manifest should contain '\\x06'");


        //     playlist_title = name_.split_once("playlist_title=").expect("manifest should contain playlist name").1.to_string();
        //     playlist_url = url_.split_once("url=").expect("manifest should contain playlist name").1.to_string();
        //     continue;
        // }

        


        let vals: Vec<&str> = entry_.split(SEP_CHAR).collect();

        let title_ = vals.get(0).unwrap();
        let id_ = vals.get(1).unwrap();
        let url_ = vals.get(2).unwrap();


        if !title_.starts_with("title=") || !id_.starts_with("id=") || !url_.starts_with("url=") {
            pl_update_fatal_error!(ErrorKind::InvalidData, "Error while parsing playlist manifest at line: {line_num}");
        }

        let title = title_.split_at("title=".len()).1.to_string();
        let id = id_.split_at("id=".len()).1.to_string();
    
        let url = if url_.len() > "url=".len() {
            Some(url_.split_at("url=".len()).1.to_string())
        } else {
            None
        };

        
        songs.push(Song::new(title, id, url));



    }

    // if line_num <= 1 {
    //     pl_update_fatal_error!(ErrorKind::UnexpectedEof, "Unexpected EOF while parsing playlist manifest.");
    // }



    Ok(songs)

}


fn download(mut urls: Vec<String>, options: &Args) -> Result<(), Error> {

    let total_songs_count = urls.len();
    
    let ffmpeg_command = options.ffmpeg_location.clone().unwrap_or("ffmpeg".to_string());
  
    find_ffmpeg(options.verbose, &ffmpeg_command)?;


    let mut max_threads = options.threads;
    
    if !options.quiet && max_threads > 1 {
        println!("Threads available: {}, Using: {}", std::thread::available_parallelism()?.get() , max_threads);
    }

    let mut output_args = vec!["--extract-audio".to_owned(),
        "--audio-format=mp3".to_owned(), "--embed-thumbnail".to_owned(), "--add-metadata".to_owned(), "--windows-filenames".to_owned()];


    if options.ffmpeg_location.is_some() {
        output_args.push("--ffmpeg-location".to_owned());
        output_args.push(options.ffmpeg_location.to_owned().unwrap());
    
    }

 

    if options.verbose {
        output_args.push("--verbose".to_owned());
    } else if options.quiet {
        output_args.push("--quiet".to_owned());
    }


    if !options.yt_dl_args.is_empty() {
        let mut user_args = options.yt_dl_args.clone();
        output_args.append(&mut user_args);
    }

    let mut urls_per_thread = (urls.len() as f32 / max_threads as f32).ceil() as usize;

    if urls.len() < 3 {
        urls_per_thread = urls.len().try_into().unwrap();
        max_threads = 1;
        pl_update_warn!("Less than three urls. Running in single thread mode.");

    } else if urls_per_thread < 3 { //If there's less than three urls per thread, decrease the thread count
        max_threads = urls.len() / 3;
        urls_per_thread = (urls.len() as f32 / max_threads as f32).ceil() as usize;
        pl_update_warn!("Less than three urls per thread, thread count decreased to {}.", max_threads);

    } 


    let mut split_url_vecs = Vec::with_capacity(max_threads.try_into().unwrap());


    let mut i = 0;
    let mut range_end;
    while i < max_threads {

        if urls.len() < urls_per_thread {
            range_end = urls.len();
        } else {
            range_end = urls_per_thread;
        }

        let thread_urls: Vec<String> = urls.drain(..range_end).collect();
        
        split_url_vecs.push(thread_urls);

        i += 1;

    } 

    if urls.len() != 0 {
        pl_update_warn!("Parser dropped {} urls: {:?}", urls.len(), urls);
    }

 
    if options.verbose {
        println!("{} [pl-update] Parsed {} URL vecs for threads {:?}", "DEBUG:".blue(), split_url_vecs.len(), split_url_vecs);
    }
    


    let mut ytdl_threads = Vec::new();
    let mut output_handlers = Vec::new();
    let mut err_handlers = Vec::new();
    
    let (tx, rx) = mpsc::sync_channel(2);

    for thread_urls in split_url_vecs {


        let mut ytdl_thread = Command::new(options.yt_dl_location.clone())
                .args([output_args.clone(), thread_urls].concat())
                .stderr(Stdio::piped())
                .stdout(Stdio::piped()) //Set ytdl to have a piped output so we can use its output later.
                .spawn()?; //Run YTDL as a child process.


        if !options.quiet {
            println!("[pl-update] Started download thread with id: {}", ytdl_thread.id());
        }

        let threadid = ytdl_thread.id();

        let output_reader = BufReader::new(ytdl_thread.stdout.take().unwrap()); //Get a handle to ytdl's output.
        let err_reader = BufReader::new(ytdl_thread.stderr.take().unwrap()); //Get a handle to ytdl's output.

        let txerr = tx.clone();
        let txout = tx.clone();

        let child_err_handler: JoinHandle<Result<usize, String>> = 
                    thread::spawn(move || parse_ytdl_stderr(err_reader, txerr, threadid));

        let child_out_handler: JoinHandle<()> =
                    thread::spawn(move ||  parse_ytdl_stdout(output_reader, txout, threadid));


        output_handlers.push(child_out_handler);
        err_handlers.push(child_err_handler);

        ytdl_threads.push(ytdl_thread);
        sleep(Duration::from_millis(200)); //Wait 1s before creating each thread to spread out the load a lil bit

    }
    
    drop(tx);


    loop {
        match rx.recv() {
            Ok(recv_string) => {
                print!("{}", recv_string);
            },
            Err(_) => {
                break;
            }

        }
    }


    let mut unavailable_songs = 0;

   


    for thread in err_handlers {
        unavailable_songs += thread.join().unwrap().unwrap();
    }

    let downloaded_songs = total_songs_count - unavailable_songs;

    for mut child in ytdl_threads {
        if !options.quiet {
           println!("[pl-update] Closed download thread with id: {}", child.id());
        }
        child.kill()?;
    }


    if !options.quiet {
        println!("[pl-update] Downloaded {} songs. {} songs were unavailable for download.", downloaded_songs, unavailable_songs);
    }

    Ok(())

}

fn fetch_settings<P: AsRef<Path>>(path: P, options: &mut Args) -> Result<(String, String, usize), Error> {
    let file = OpenOptions::new().read(true).open(path)?;
    let settings = PlaylistSettings::from_file(file)?;
    if options.postproccessor_args.is_empty() && !settings.postprocessor_args.is_empty() {
        options.postproccessor_args = settings.postprocessor_args;
    }

    if options.yt_dl_args.is_empty() && !settings.yt_dl_args.is_empty() {
        options.yt_dl_args = settings.yt_dl_args;
    }

    Ok((settings.playlist_name, settings.playlist_url, settings.backup_manifest_count.unwrap_or(7)))
}

fn file_is_manifest(filename: &Utf16String) -> bool {
    if (filename.starts_with("playlist".into()) && filename.ends_with(".manifest".into())) || filename == &Utf16String::from_str("playlist-settings.json") {
        true
    } else {
        false
    }
}







