
use std::{io::{BufRead, BufReader, Error, ErrorKind}, process::{ChildStderr, ChildStdout, Command, Stdio}, sync::mpsc::{self, SyncSender}, thread::{self, sleep, JoinHandle}, time::Duration};

use colored::Colorize;

use crate::{debug_print, error_print, stdout_print, warn_print};

use super::App;

impl App {
    pub(super) fn parse_ytdl_stderr(mut std_err_reader: BufReader<ChildStderr>, tx: SyncSender<String>, procid: u32) -> Result<usize, String> {
        let mut err_str: String = String::new();
        let mut err_bytes_read = 1;
        let mut unavailable_songs: usize = 0;

            while err_bytes_read > 0 {


                
                let err_val = std_err_reader.read_line(&mut err_str);

                let out_str;

                if err_val.is_err() {
                    error_print!("Read from YT-DL STDERR buffer failed with error: \"{}\"!", err_val.unwrap_err());
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



    pub(super) fn download(&mut self, mut urls: Vec<String>) -> Result<(), Error> {

        let total_songs_count = urls.len();
        let mut max_threads = self.args.threads;
        
        if !self.args.quiet && max_threads > 1 {
            stdout_print!("Threads available: {}, Using: {}", std::thread::available_parallelism()?.get() , max_threads);
        }

        let mut output_args = vec!["--extract-audio".to_owned(),
            "--audio-format=mp3".to_owned(), "--embed-thumbnail".to_owned(), "--add-metadata".to_owned(), "--windows-filenames".to_owned()];


    

        if self.args.verbose {
            output_args.push("--verbose".to_owned());
        } else if self.args.quiet {
            output_args.push("--quiet".to_owned());
        }


        if !self.args.yt_dl_args.is_empty() {
            let mut user_args = self.args.yt_dl_args.clone();
            output_args.append(&mut user_args);
        }

        let mut urls_per_thread = (urls.len() as f32 / max_threads as f32).ceil() as usize;

        if urls.len() < 3 {
            urls_per_thread = urls.len().try_into().unwrap();
            max_threads = 1;
            warn_print!("Less than three urls. Running in single thread mode.");

        } else if urls_per_thread < 3 { //If there's less than three urls per thread, decrease the thread count
            max_threads = urls.len() / 3;
            urls_per_thread = (urls.len() as f32 / max_threads as f32).ceil() as usize;
            warn_print!("Less than three urls per thread, thread count decreased to {}.", max_threads);

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
            warn_print!("Parser dropped {} urls: {:?}", urls.len(), urls);
        }

    
        if self.args.verbose {
            debug_print!("Parsed {} URL vecs for threads {:?}", split_url_vecs.len(), split_url_vecs);
        }
        


        let mut ytdl_threads = Vec::new();
        let mut output_handlers = Vec::new();
        let mut err_handlers = Vec::new();
        
        let (tx, rx) = mpsc::sync_channel(2);

        for thread_urls in split_url_vecs {


            let mut ytdl_thread = Command::new(self.args.yt_dl_location.clone())
                    .args([output_args.clone(), thread_urls].concat())
                    .stderr(Stdio::piped())
                    .stdout(Stdio::piped()) //Set ytdl to have a piped output so we can use its output later.
                    .spawn()?; //Run YTDL as a child process.


            if !self.args.quiet {
                stdout_print!("Started download thread with id: {}", ytdl_thread.id());
            }

            let threadid = ytdl_thread.id();

            let output_reader = BufReader::new(ytdl_thread.stdout.take().unwrap()); //Get a handle to ytdl's output.
            let err_reader = BufReader::new(ytdl_thread.stderr.take().unwrap()); //Get a handle to ytdl's output.

            let txerr = tx.clone();
            let txout = tx.clone();

            let child_err_handler: JoinHandle<Result<usize, String>> = 
                        thread::spawn(move || Self::parse_ytdl_stderr(err_reader, txerr, threadid));

            let child_out_handler: JoinHandle<()> =
                        thread::spawn(move ||  Self::parse_ytdl_stdout(output_reader, txout, threadid));


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
            if !self.args.quiet {
            stdout_print!("Closed download thread with id: {}", child.id());
            }
            child.kill()?;
        }


        if !self.args.quiet {
            stdout_print!("Downloaded {} songs. {} songs were unavailable for download.", downloaded_songs, unavailable_songs);
        }

        Ok(())

    }

}