use std::{fs::read_dir, io::{self, BufReader, Error, ErrorKind}, process::{Command, Stdio}, sync::mpsc, thread::{self, JoinHandle}};
use colored::Colorize;
use widestring::{Utf16String, WideUtfString};

use crate::{app::YOUTUBE_ID_LEN, debug_print, fatal_error, stdout_print, string_parsing::{ReadtoUtf16String, StringExts}, warn_print, FILE_EXT, SEP_CHAR};

use super::{App, PLAYLIST_SETTINGS_NAME};

use crate::Song;


impl App {

    pub(super) fn fetch_manifest_local(&self) -> Result<Vec<Song>, Error> {

    
        macro_rules! cnd_print_debug {
            ($($x:tt)*) => {
                if self.args.verbose {
                    debug_print!(
                        $(
                            $x
                        )*
                    );
                }
            };
        }

        macro_rules! cnd_print_stdout {
            ($($x:tt)*) => {
                if !self.args.quiet {
                    stdout_print!(
                    $(
                        $x
                    )*
                );
                }
            };
        }


        cnd_print_stdout!("Fetching contents of playlist \"{}\" from disk...", self.settings.playlist_name);

        
        // This list contains all files in the target directory.
        let directory_entry = read_dir(".")?.collect::<Result<Vec<_>, io::Error>>().unwrap();

        // at least one file in the directory should be the playlist settings, so this is the best guess for alloc size
        let mut songs = Vec::with_capacity(directory_entry.len()-1);
    
        for file_entry in directory_entry {

            let file_name = WideUtfString::from_os_string(file_entry.file_name()).unwrap();

            

            if Self::file_is_manifest(&file_name) { // skip manifests without warning
                continue;
            }

            if !file_name.ends_with(FILE_EXT.into()) {
                warn_print!("Loose file \"{}\" in directory.",  file_name);
                continue;
            }


            if let Some((song_name, remainder)) = file_name.rsplit_once("[".into()) {// If it has an opening bracket
                
                if remainder.len() != FILE_EXT.len() + YOUTUBE_ID_LEN + 2 {
                    warn_print!("Non yt-dl file \"{}\" in directory.", file_name);
                    continue;
                }

                let song = Song::new(remainder[1..remainder.len() - FILE_EXT.len() - 1].to_owned(), song_name[0..song_name.len()-1].to_owned(), None);
                cnd_print_debug!("Found {:?}", song);
                songs.push(song);

                
            } else {
                warn_print!("Non yt-dl file \"{}\" in directory.", file_name);
                continue;

            } 

        

        }
    
        Ok(songs)
    
    }

    
    pub(super) fn fetch_manifest_url(&self, playlist_title: &String, playlist_url: &String) -> Result<Vec<Song>, Error> {
    
        let mut output_args = Vec::new();
        let command_name = &self.args.yt_dl_location;
    
        if self.args.verbose {
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
    
    
        if !self.args.quiet {
            stdout_print!("Fetching contents of playlist \"{playlist_title}\" from remote...");
        } 
    
    
        if self.args.verbose {
            debug_print!("Running {} with arguments {:?}", command_name, output_args);
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
        let verbose = self.args.verbose;
        let ytdl_err_handler = thread::spawn(move || Self::parse_ytdl_stderr(err_reader, tx, procid));
    
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
                    fatal_error!(ErrorKind::InvalidData, "Error while parsing playlist manifest at line: {line_num}");
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
            print!("{}", rx.recv().unwrap_or("".to_string()));
        }
        
        let songs = ytdl_out_handler.join().unwrap()?;

        if !self.args.quiet {
            stdout_print!("Successfully found {} items in remote playlist.", songs.len())
        }

        Ok(songs)


    }
    
   
    pub(super) fn file_is_manifest(filename: &Utf16String) -> bool {
        if filename == &Utf16String::from_str(PLAYLIST_SETTINGS_NAME) {
            true
        } else {
            false
        }
    }


}