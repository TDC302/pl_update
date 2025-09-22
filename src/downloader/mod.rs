use std::{io::{stderr, BufRead, BufReader, ErrorKind, Read, Write}, process::{Command, Stdio}, str::FromStr, sync::mpsc::{self, SyncSender}, thread::{self, sleep, JoinHandle}, time::Duration, usize};

use colored::Colorize;
use const_format::formatcp;

use crate::{app::YOUTUBE_ID_LEN, debug_print, downloader::error::Error, error_print, stdout_print, string_parsing::ReadtoUtf16String, warn_print, Song, SEP_CHAR};


pub mod error;

const NA_PLACEHOLDER: &str = "\x15";

pub struct Downloader {
    version: String,
    pathspec: String,
    downloader_args: Vec<String>
}

impl Downloader {
    pub fn new(command: String, args: Vec<String>) -> Result<Self, Error> {

        match std::process::Command::new(&command).arg("--version").output() {
            Ok(v) => {
                let ver = std::str::from_utf8(&v.stdout).unwrap().trim();
                Ok(Self {version: ver.to_owned(), pathspec: command, downloader_args: args})
            }, 
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    Err(Error::InitializationError(command))
                } else {
                    Err(Error::IoError(e))
                }
            }
        }
      
    }

    pub fn version(&self) -> &str {
        &self.version
    }


    pub fn fetch_playlist_content(&self, playlist_url: &String, verbose: bool) -> Result<Vec<Song>, Error> {
    
        let mut output_args = vec![
            "--restrict-filenames", "--windows-filenames",
            "--simulate", "--flat-playlist", "--lazy-playlist",
            playlist_url,
            "--print", formatcp!("title=%(title)s{SEP_CHAR}id=%(id)s{SEP_CHAR}url=%(webpage_url)s")];
    
        if verbose {
            output_args.push("--verbose");
            output_args.push("--quiet"); // using both prints log messages to stderr
        }
    
        if verbose {
            debug_print!("Running {} with arguments {:?}", self.pathspec, output_args);
        }
      
    
        let mut ytdl_process = Command::new(&self.pathspec)
        .args(&output_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    
        let err_reader = BufReader::new(ytdl_process.stderr.take().unwrap());
        let mut out_reader = BufReader::new(ytdl_process.stdout.take().unwrap());
        let (tx, rx) = mpsc::sync_channel(2);
        let procid = ytdl_process.id();
        let ytdl_err_handler = thread::spawn(move || Self::forward_output(err_reader, tx, procid));
    
        let ytdl_out_handler: JoinHandle<Result<Vec<Song>, Error>> = thread::spawn(move || {

            let mut line_num = 0;
            let mut songs = Vec::new();
            let mut buf = String::new();


            while out_reader.read_line_lossy(&mut buf)? > 0 {
                
                line_num += 1;
        
                let vals: Vec<&str> = buf.split(SEP_CHAR).collect();
        
                let title_ = vals.get(0).unwrap();
                let id_ = vals.get(1).unwrap();
                let url_ = vals.get(2).unwrap();
        
        
                if !title_.starts_with("title=") || !id_.starts_with("id=") || !url_.starts_with("url=") {
                    return Err(Error::ParserError("playlist manifest".to_owned(), format!("Line {line_num}")));
                }
        
                let title = title_.split_at("title=".len()).1.to_string();
                let id = id_.split_at("id=".len()).1.to_string();
            
                let url = if url_.len() > "url=".len() {
                    Some(url_.split_at("url=".len()).1.to_string())
                } else {
                    None
                };
        
                if verbose {
                    debug_print!("Found playlist item title: {}, id: {}, url: {:?}", title, id, url);
                }
                
                songs.push(Song::new_str(title, id, url));
                
                buf.clear();

            }
            
            Ok(songs)
    
        });

       

        while !ytdl_err_handler.is_finished() || !ytdl_out_handler.is_finished() {
            match rx.recv() {
                Ok((_, Err(e))) => error_print!("{e}"),
                Ok((_, Ok(m))) => stdout_print!("{m:?}"),
                Err(_) => break
            } 
        }
        
        let songs = ytdl_out_handler.join().unwrap()?;

        Ok(songs)


    }


    pub fn fetch_playlist_name(&self, playlist_url: &String, verbose: bool) -> Result<String, Error> {
        // all of this just to fetch the name of the playlist...
        let mut output_args = vec!["--simulate", "--flat-playlist",
                "--lazy-playlist", playlist_url, "--print", "%(playlist)s", "--playlist-items=1",
                formatcp!("--output-na-placeholder={NA_PLACEHOLDER}")];

        if verbose {
            output_args.push("--verbose");
        }
       
        

        //cnd_print_debug!("Running {} with arguments {:?}", command_name, output_args);

        let ytdl_output = Command::new(&self.pathspec)
                .args(output_args.clone())
                .output()?;



        
        let playlist_name = str::from_utf8(&ytdl_output.stdout).expect("output should be valid utf-8").trim();
        
        stderr().write_all(&ytdl_output.stderr)?;

        
        if playlist_name == NA_PLACEHOLDER {
            return Err(Error::PlaylistDoesNotExist)
        } else {
            return Ok(playlist_name.to_owned());
        }
    }



    pub(super) fn download(&mut self, mut urls: Vec<String>, max_threads: usize, progress: bool, verbose: bool) -> Result<Vec<(String, Result<(), Error>)>, Error> {
                                                                                    // ^^^^^^^^^^^^^ This needs to be moved to some flags enum or something
        let total_songs_count = urls.len();
        let mut threads = max_threads;
     
        let mut output_args = vec!["--extract-audio".to_owned(),
            "--audio-format=mp3".to_owned(), "--embed-thumbnail".to_owned(), 
            "--add-metadata".to_owned(), "--windows-filenames".to_owned(), 
            "--quiet".to_owned(),
                // When a file is finished, will print something like
                // s|sj4mfQ1nLb|This is not a real video!
            "--print".to_owned(), format!("after_move:s{SEP_CHAR}%(id)s{SEP_CHAR}%(title)s"),
            // with the print option specified yt-dlp defaults to simulatiing downloads
            "--no-simulate".to_owned()];

            if progress {
                // will output something like p|sj4mfQ1nLb|This is not a real video!|584838.433|3242|4832929
                output_args.push("--progress".to_owned());
                output_args.push("--progress-template".to_owned());
                output_args.push(format!("p{SEP_CHAR}%(info.id)s{SEP_CHAR}%(info.title)s{SEP_CHAR}%(progress.speed)s{SEP_CHAR}%(progress.downloaded_bytes)s{SEP_CHAR}%(progress.total_bytes)s"));
                output_args.push("--newline".to_owned());
            }


    

        if verbose {
            output_args.push("--verbose".to_owned());
            // using both verbose and quiet prints log to stderr
        }


        if !self.downloader_args.is_empty() {
            let mut user_args = self.downloader_args.clone();
            output_args.append(&mut user_args);
        }

        let mut urls_per_thread = (urls.len() as f32 / threads as f32).ceil() as usize;

        if urls.len() < 3 {
            urls_per_thread = urls.len().try_into().unwrap();
            threads = 1;
            warn_print!("Less than three urls. Running in single thread mode.");

        } else if urls_per_thread < 3 { //If there's less than three urls per thread, decrease the thread count
            threads = urls.len() / 3;
            urls_per_thread = (urls.len() as f32 / threads as f32).ceil() as usize;
            warn_print!("Less than three urls per thread, thread count decreased to {}.", threads);

        } 


        let mut split_url_vecs = Vec::with_capacity(threads.try_into().unwrap());


        let mut i = 0;
        let mut range_end;
        while i < threads {

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
            warn_print!("Parser dropped {} urls: {:?}", urls.len(), urls);
        }

    
        if verbose {
            debug_print!("Parsed {} URL vecs for threads {:?}", split_url_vecs.len(), split_url_vecs);
        }
        


        let mut ytdl_threads = Vec::new();
        let mut output_handles = Vec::new();
        
        let (tx, rx) = mpsc::sync_channel(threads);

        for thread_urls in split_url_vecs {


            let mut ytdl_thread = Command::new(&self.pathspec)
                    .args([output_args.clone(), thread_urls].concat())
                    .stderr(Stdio::piped())
                    .stdout(Stdio::piped()) 
                    .spawn()?; 
            

            let threadid = ytdl_thread.id();

            let stderr = ytdl_thread.stderr.take().unwrap();
            let stdout = ytdl_thread.stdout.take().unwrap();
            let txerr = tx.clone();
            let txout = tx.clone();

            // the purpose of forwarding output here is so that it all ends up on one reciever which will block while waiting for data
            // otherwise we'd have to spin on bufreaders, or block on the bufreader for one process while others might have output
            output_handles.push(thread::spawn(move || 
                Self::forward_output(stdout, txout, threadid)));

            output_handles.push(thread::spawn(move ||  
                Self::forward_output(stderr, txerr, threadid)));

            ytdl_threads.push(ytdl_thread);
            sleep(Duration::from_millis(200)); //Wait 1s before creating each thread to spread out the load a lil bit

        }
        
        drop(tx); // this sender is leftover, and if we don't drop it our reciever will never reach eof
        
        let mut result = Vec::with_capacity(total_songs_count);

        loop {
            match rx.recv() {
                Ok((t_id, Ok(msg))) => {
                    match msg {
                        DownloaderMessage::Success { id, title } => {
                            stdout_print!("[download {t_id}] Finished downloading '{title}'");
                            result.push((id, Ok(())));
                        },
                        DownloaderMessage::Progress { id: _, title, speed, bytes_recieved, bytes_total } => {
                            print!("{title}: {:5.2}% of {:.2}mb at {:.2}kb/s\t\t\r", 
                            (bytes_total/bytes_recieved)*100, (bytes_total as f32)/1_000_000f32, speed/1_000f32);
                        },
                        DownloaderMessage::Error { id, description } => {
                            if let Some(idu) = id {
                                error_print!("[download {t_id}] Video {idu}: {description}");
                                result.push((idu, Err(Error::DowloaderError(description))));
                            } else {
                                error_print!("[download {t_id}] {description}");
                            }
                        }
                    }  
                },
                Ok((id, Err(e))) => {
                    error_print!("[download {id}] {e:?}")
                }
                Err(_) => {
                    break;
                }

            }
        }


        for mut child in ytdl_threads {
            child.kill()?;
        }

        Ok(result)

    }


   

    fn forward_output<T: Read>(source: T, target: SyncSender<(u32, Result<DownloaderMessage, Error>)>, id: u32) -> () {
        let mut reader = BufReader::new(source);
        let mut buf: String = String::new();
        let mut bytes_read = 1;

        while bytes_read > 0 {
            let result = reader.read_line(&mut buf);

            match result {
                Err(e) => error_print!("Buffer read error on process {id}: \"{}\"!", e),
                Ok(b) => {
                    bytes_read = b;

                    if !buf.trim().is_empty() { // sometimes ytdlp outputs newlines with no data
                        target.send((id, buf.parse())).unwrap();
                    }
                },

            } 
            buf.clear();

        }
    }
    
}

#[derive(Debug)]
enum DownloaderMessage {
    /// The video downloaded and converted successfully
    Success {
        id: String,
        title: String
    },

    /// The video is downloading
    Progress {
        id: String,
        title: String,
        speed: f32,
        bytes_recieved: usize,
        bytes_total: usize
    },

    Error {
        id: Option<String>,
        description: String
    }
}

macro_rules! downloader_msg_error {
    ($($x:expr),*) => {
        Err(Error::ParserError("downloader message".to_owned(),
        format! (
                $(
                    $x,
                )*
            )
        ))
    };
}

impl DownloaderMessage {
    fn parse_raw_error_msg(s: &str) -> Result<Self, Error> {
        let mut msg = s.to_owned();
        
        let desc;
        msg = msg.split_off("ERROR: ".len());
        if msg.starts_with("[youtube] ") {
            msg = msg.split_off("[youtube] ".len());
            let desc = msg.split_off(YOUTUBE_ID_LEN);

            Ok(Self::Error { id: Some(msg), description: desc})

        } else if msg.starts_with("[generic] ") {
            desc = msg.split_off("[generic] ".len());

            Ok(Self::Error { id: None, description: desc })

        } else {
            downloader_msg_error!("unknown format {s}")
        }

    }


}


impl FromStr for DownloaderMessage {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("ERROR: ") {
            return Self::parse_raw_error_msg(s);
        }

        let mut sp = s.split(SEP_CHAR);
        let msg_type = sp.next();
        let id = sp.next();
        let title = sp.next();

        // this is an iterator so we only need to check if the last item exists
        if title.is_none() || msg_type.unwrap().len() != 1 { 
            return downloader_msg_error!("unknown format: '{s}'");
        }

       


        match msg_type.unwrap() {
            "s" => Ok(Self::Success { id: id.unwrap().to_owned(), title: title.unwrap().trim().to_owned() }),
            "p" => {
                let speed_str = sp.next();
                let bytes_rcv_str = sp.next();        
                let bytes_total_str = sp.next();
            

                if bytes_total_str.is_some() {
                    Ok(Self::Progress { id: id.unwrap().to_string(), title: title.unwrap().to_string(), 
                        speed: if speed_str.unwrap() == "NA" {
                            0.0
                        } else {
                            speed_str.unwrap().parse::<f32>().map_err(|e| 
                                Error::ParserError("downloader message".to_owned(), format!("floating point conversion error {e} while parsing '{s}'")))?
                        },
                          
                        bytes_recieved: bytes_rcv_str.unwrap().parse::<usize>().map_err(|e| 
                            Error::ParserError("downloader message".to_owned(), format!("usize conversion error '{e}' while parsing '{s}'")))?,
                
                        bytes_total: bytes_total_str.unwrap().trim().parse::<usize>().map_err(|e| 
                            Error::ParserError("downloader message".to_owned(), format!("usize conversion error '{e}' while parsing '{s}'")))?,
                    })
                } else {
                    downloader_msg_error!("unknown format: '{s}'")
                }

               
            }
            _ => downloader_msg_error!("unknown format: '{s}'")
        }

        
    }

}
