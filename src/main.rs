extern crate chrono;
extern crate os_info;


mod push;
mod adb;
mod init;
mod update;
mod repair;


use core::str;
use std::fmt::Debug;
use std::{env, thread};
use std::fs::File;
use std::io::{BufRead, BufReader, Error, Read, Write};
use std::process::{ChildStderr, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Sender};
use std::thread::{sleep, JoinHandle};
use std::time::{Duration, SystemTime};
use chrono::Local;
use clap::{Parser, Subcommand};
use colored::Colorize;



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
    ($($x:expr),*) => {

        let emsg = format!(
            $(
                $x,
            )*
        );
        eprintln!("\n{} [pl-update] {}\n In: {} Line: {}\n", "FATAL ERROR:".red().bold(), emsg, file!(), line!());
        
        return Err(Error::new(std::io::ErrorKind::Other, emsg));
        
    
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
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    // /// The id of the device to push to
    // #[arg(short, long)]
    // device_id: String,

    /// Print extra debugging information
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Suppress output
    #[arg(short, long, default_value_t = false)]
    quiet: bool,


    /// Args to pass to yt-dlp
    #[arg(long)]
    yt_dl_args: Vec<String>,


    /// The location of yt-dlp
    #[arg(long, default_value_t = {"yt-dlp".to_string()})] 
    yt_dl_location: String,


    
    /// The location of yt-dlp
    #[arg(long, default_value_t = {"ffmpeg".to_string()})] 
    ffmpeg_location: String,


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
    Init { playlist_url: String },
    Update { playlist_name: Option<String> },
    Repair { playlist_name: Option<String> },
    Push { device_id: Option<String> }
}


#[derive(Debug, Clone)]
struct Song {
    title: String,
    id: String,
    url: Option<String>,
}

impl PartialEq for Song {
    fn eq(&self, other: &Self) -> bool {
        self.title == other.title && self.id == other.id
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

fn main() -> std::io::Result<()> {
    let time: chrono::DateTime<Local> = SystemTime::now().into();
    let info = os_info::get();

    const SW_MAJOR: u16 = 3;
    const SW_MINOR: u16 = 0;
    const SW_PATCH: u32 = 0;

    

    env::set_var("RUST_BACKTRACE", "1");


    
    let args = Args::parse();
    
    macro_rules! pl_update_println {
        ($($x:expr),*) => {
            if !args.quiet {
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



    pl_update_println!("pl_update version {SW_MAJOR}.{SW_MINOR}.{SW_PATCH}");
    
    if info.architecture().is_some() {
        pl_update_println!("Running on {} {} for {}", info.os_type(), info.version(), info.architecture().unwrap());
    } else {
        pl_update_println!("Running on {} version {}", info.os_type(), info.version());
    }

    pl_update_println!("Started at system time {}\n", time.format("%+"));





    
    pl_update_vprintln!("Args: {:?}", args);

    let command = args.command.clone();
 

    let ret = match command {
        //Commands::Get => todo!(),
        Commands::Init { playlist_url } => init::pl_init(args, playlist_url),
        Commands::Push { device_id } => push::pl_push(args, device_id),
        Commands::Repair { playlist_name } => repair::pl_repair(args, playlist_name),
        Commands::Update { playlist_name } => update::pl_update(args, playlist_name)

    };

    let new_time: chrono::DateTime<Local> = SystemTime::now().into();

    let delta = new_time - time;


    match ret {
        Ok(_) => {
            println!("[pl-update] Operation completed in {}m {}s", delta.num_minutes(), delta.num_seconds());


            Ok(())
        
        },
        Err(e) => {
            Err(e)
        }
    }


    
 

}


// fn print_usage(){
//     println!("\nUSAGE: pl-update.exe [OPTIONS] URL\n");
//     println!("Type \"pl-update.exe --help\" for more detailed information.")
    


// }

// fn print_help(){
//     println!("\nUSAGE: pl-update.exe [OPTIONS] URL\n");

//     println!("Options:");
//     println!("\t-h, --help\t\t\t\t\t\tPrint this help text and exit");
//     println!("\t-v, --verbose\t\t\t\t\t\tPrint various debugging information");
//     println!("\t-q, --quiet\t\t\t\t\t\tSuppresses output to warnings and errors only. Not available with --verbose");
//     println!("\t--push-to-device\t\t\t\t\tPushes downloaded songs to connected android device.");
    

// }


// fn process_args(args: Vec<String>) -> std::io::Result<(CommandArg, OptionArgs)> {

//     //args.remove(0); // The arg at index 0 is the program name and can be removed.

//     let mut command = String::from_str("").unwrap();
//     let mut options: Vec<String> = Vec::new();

    

//     let mut i = -1;
//     for arg in args {
//         i += 1;
//         if i == 0 {
//             continue;
//         } else if arg == "" {
//             continue;
//         } else if arg.starts_with("-") {
//             options.push(arg);
//             continue;
            
//         } else if command == "" {
//             command = arg;
//             continue;
            
//         } else {
//             pl_update_fatal_error!("Argument \"{}\" is not recognised.", arg);
//         }
        
//     }


//     let res_opt = OptionArgs::from_string_vec(options);
//     let ret_opt;

//     match res_opt {
//         Ok(val) => ret_opt = val,
//         Err(e) => {pl_update_fatal_error!("{}", e);}
//     }

    

//     if command == "" && ret_opt.help {
//         print_help();
//         pl_update_ok_exit!();
//     } else if command == "" {
//         print_usage();
//         pl_update_fatal_error!("Command not specified!");
//     }
 
//     let res_cmd = CommandArg::from_string(command.clone());
//     let ret_cmd;

//     match res_cmd {
//         Ok(val) => ret_cmd = val,
//         Err(()) => {pl_update_fatal_error!("Command \"{}\" not recognised.", command);}
//     }



//     if ret_opt.verbose && ret_opt.quiet {
//         print_usage();
//         pl_update_fatal_error!("The --quiet option cannot be specified with the --verbose option.");
//     }




//     Ok((ret_cmd, ret_opt))

// }

fn find_yt_dl(verbose: bool, ytdl_command: &String) -> Result<(), Error> {

    //let prog_names = ["yt-dlp", "youtube-dl", "yt-dl"];

   
    let ytdl_check: Result<std::process::Output, Error> = Command::new(ytdl_command.clone()).arg("--version").output();
    
    if ytdl_check.is_ok() {
        let out = &ytdl_check.unwrap().stdout;
        let ver = str::from_utf8(out).unwrap().trim();
        if verbose {
            println!("{} [pl-update] Found {} version {}", "DEBUG:".blue(), ytdl_command, ver);
        }
        return Ok(());
        
    } 
        
    
    

    pl_update_fatal_error!("YT-DL could not be found. Check that it is in the system path or current directory and is accessible.");
    

    //command_name.unwrap().to_string();
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

    pl_update_fatal_error!("YT-DL could not be found. Check that it is in the system path or current directory and is accessible.");


}

fn parse_ytdl_stderr(mut std_err_reader: BufReader<ChildStderr>, tx: Sender<String>, procid: u32) -> Result<i32, String> {
    let mut err_str: String = String::new();
    let mut err_bytes_read = 1;
    let mut unavailable_songs = 0;

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



            if err_str.starts_with("[debug] ") {
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

    return Ok(unavailable_songs);


}

fn parse_ytdl_stdout(mut std_out_reader: BufReader<ChildStdout>, tx: Sender<String>, procid: u32) -> (i32, i32) {
    

    let mut bytes_read = 1;
    let mut buffer_str: String = String::new();
    let mut songs_in_playlist = -1;
    let mut skipped_songs = 0;

    while bytes_read > 0 { //When the output reader returns 0 bytes read, we know ytdl is done.

        
        

        
        bytes_read = std_out_reader.read_line(&mut buffer_str).unwrap(); //Read a line from YTDL's output.

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
            out_str = format!("[thread {}] {}", procid, buffer_str);

        }
        
        tx.send(out_str).unwrap();
        buffer_str.clear();


    }
    (songs_in_playlist, skipped_songs)


}

fn update_manifest(mut manifest: File, playlist_title: String, playlist_url: &String, options: &Args) -> Result<(), Error> {

    manifest.write(format!("playlist_title={}{SEP_CHAR}url={}\n", playlist_title, playlist_url).as_bytes())?;
    
    let mut output_args = Vec::new();
    let command_name = &options.yt_dl_location;

    if options.verbose {
        output_args.push("--verbose".to_owned());
        output_args.push("--quiet".to_owned());
    }

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
    let (tx, rx) = mpsc::channel();
    let procid = ytdl_process.id();
    let ytdl_err_handler = thread::spawn(move || parse_ytdl_stderr(err_reader, tx, procid));

    let ytdl_out_handler: JoinHandle<Result<(),Error>> = thread::spawn(move || {
    let buf = &mut vec![];
    out_reader.read_to_end(buf)?;

    manifest.write_all(&buf)?;

    Ok(())

    });

    while !ytdl_err_handler.is_finished() || !ytdl_out_handler.is_finished() {
        print!("{}", rx.recv().unwrap_or("".to_string()));
    }

    

    Ok(())
}

fn parse_manifest(manifest: File) -> Result<(Vec<Song>, String, String), Error> {

    let mut playlist_title = "".to_string();
    let mut playlist_url = "".to_string();

  

    let file_reader = BufReader::new(manifest);

    let entries = file_reader.lines();

    let mut songs = Vec::new();


    let mut line_num = 0;
    for entry in entries {
        line_num += 1;


        let entry_ = entry?;
  

        if line_num == 1 {
            let first: Vec<&str> = entry_.split(SEP_CHAR).collect();
            
            let name_ = first.get(0).expect("manifest should contain '\\x06'");
            let url_ = first.get(1).expect("manifest should contain '\\x06'");


            playlist_title = name_.split_once("playlist_title=").expect("manifest should contain playlist name").1.to_string();
            playlist_url = url_.split_once("url=").expect("manifest should contain playlist name").1.to_string();
            continue;
        }

        
    

        

        let vals: Vec<&str> = entry_.split(SEP_CHAR).collect();

        let title_ = vals.get(0).unwrap();
        let id_ = vals.get(1).unwrap();
        let url_ = vals.get(2).unwrap();


        if !title_.starts_with("title=") || !id_.starts_with("id=") || !url_.starts_with("url=") {
            pl_update_fatal_error!("Error while parsing playlist manifest at line: {line_num}");
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

    if line_num <= 1 {
        pl_update_fatal_error!("Unexpected EOF while parsing playlist manifest.");
    }



    Ok((songs, playlist_title, playlist_url))

}


fn download(mut urls: Vec<String>, options: &Args) -> Result<(), Error> {

    

    let mut max_threads = options.threads;
    
    if !options.quiet && max_threads > 1 {
        println!("Cores available: {}, Using: {}", std::thread::available_parallelism()?.get() , max_threads);
    }

    let mut output_args = vec!["--extract-audio".to_owned(),
        "--audio-format=mp3".to_owned(), "--embed-thumbnail".to_owned(), "--add-metadata".to_owned()];


    output_args.push("--ffmpeg-location".to_owned());
    output_args.push(options.ffmpeg_location.to_owned());

 

    if options.verbose {
        output_args.push("--verbose".to_owned());
    } else if options.quiet {
        output_args.push("--quiet".to_owned());
    }


    if !options.yt_dl_args.is_empty() {
        let mut user_args = options.yt_dl_args.clone();
        output_args.append(&mut user_args);
    }

    let mut urls_per_thread = urls.len() / max_threads;

    
    if urls.len() < 3 {
        urls_per_thread = urls.len();
        max_threads = 1;
        pl_update_warn!("Less than three urls. Running in single thread mode.");

    } else if urls_per_thread < 3 { //If there's less than three urls per thread, decrease the thread count
        max_threads = urls.len() / 3;
        urls_per_thread = urls.len() / max_threads;
        pl_update_warn!("Less than three urls per thread, thread count decreased to {}.", max_threads);

    } 


    let mut split_url_vecs = Vec::with_capacity(max_threads);


    let mut i = 0;
    let mut range_end;
    while i < max_threads {

        
        if urls.len() % urls_per_thread == 0 {
            range_end = urls_per_thread;
        } else {
            range_end = urls_per_thread + 1;
        }

        let thread_urls: Vec<String> = urls.drain(..range_end).collect();
        
        split_url_vecs.push(thread_urls);

        i += 1;

    } 

    if urls.len() != 0 {
        pl_update_warn!("Parser is dumb and dropped {} urls, sorry.", urls.len());
    }

 
    if options.verbose {
        println!("{} [pl-update] URL vecs for threads {:?}", "DEBUG:".blue(), split_url_vecs);
    }
    

    let (tx, rx) = mpsc::channel::<String>();

    let mut ytdl_threads = Vec::new();
    let mut output_handlers = Vec::new();
    let mut err_handlers = Vec::new();

    for thread_urls in split_url_vecs {


        let mut ytdl_thread = Command::new(options.yt_dl_location.clone())
                .args([output_args.clone(), thread_urls].concat())
                .stderr(Stdio::piped())
                .stdout(Stdio::piped()) //Set ytdl to have a piped output so we can use its output later.
                .spawn()?; //Run YTDL as a child process.


        if options.verbose {
            println!("{} [pl-update] Started download thread with id: {}", "DEBUG:".blue(), ytdl_thread.id());
        }

        let threadid = ytdl_thread.id();

        let output_reader = BufReader::new(ytdl_thread.stdout.take().unwrap()); //Get a handle to ytdl's output.
        let err_reader = BufReader::new(ytdl_thread.stderr.take().unwrap()); //Get a handle to ytdl's output.

        let txerr = tx.clone();
        let txout = tx.clone();

        let child_err_handler: JoinHandle<Result<i32, String>> = 
                    thread::spawn(move || parse_ytdl_stderr(err_reader, txerr, threadid));

        let child_out_handler: JoinHandle<(i32,i32)> =
                    thread::spawn(move ||  parse_ytdl_stdout(output_reader, txout, threadid));


        output_handlers.push(child_out_handler);
        err_handlers.push(child_err_handler);

        ytdl_threads.push(ytdl_thread);
        sleep(Duration::from_secs(1)); //Wait 1s before creating each thread to spread out the load a lil bit

    }


    let mut recv = rx.recv();
    while recv.is_ok() {
        print!("{}", recv.as_ref().unwrap());
        recv = rx.recv();

    }

    let mut songs_in_playlist = 0;
    let mut unavailable_songs = 0;

    for thread in output_handlers {
        songs_in_playlist += thread.join().unwrap().0;
    }

    for thread in err_handlers {
        unavailable_songs += thread.join().unwrap().unwrap();
    }

    for mut child in ytdl_threads {
        if options.verbose {
            println!("{} [pl-update] Closed download thread with id: {}", "DEBUG:".blue(), child.id());
        }
        child.kill()?;
    }


    if !options.quiet {
        println!("[pl-update] Downloaded {} songs. {} songs were unavailable for download.", songs_in_playlist, unavailable_songs);
    }

    Ok(())

}


