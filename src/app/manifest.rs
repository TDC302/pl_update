use std::{env::current_dir, fs::File, io::{self, BufRead, BufReader, Error, ErrorKind, Read, Write}, process::{Command, Stdio}, sync::mpsc, thread::{self, JoinHandle}};
use colored::Colorize;
use widestring::Utf16String;

use crate::{string_parsing::StringExts, warn_print, SEP_CHAR, stdout_print, debug_print};
use crate::fatal_error;

use super::App;

use crate::Song;


impl App {

    pub(super) fn fetch_manifest_local(manifest: File) -> Result<Vec<Song>, Error> {

        let file_reader = BufReader::new(manifest);
    
        let entries = file_reader.lines();
    
        let mut songs = Vec::new();
    
        let mut line_num = 0;
        for entry in entries {
            line_num += 1;
    
            let entry_;
    
    
            if let Err(e) = entry {
                if e.kind() == ErrorKind::InvalidData {
                    warn_print!("Invalid data while parsing manifest line {line_num}!");
                    continue;
                } else {
                    return Err(e);
                }
            } else {
                entry_ = entry.unwrap();
            }
    
    
            let vals: Vec<&str> = entry_.split(SEP_CHAR).collect();
    
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
    
            
            songs.push(Song::new(title, id, url));
    
    
    
        }
    
    
        Ok(songs)
    
    }

    
    pub(super) fn fetch_manifest_url(&self, mut manifest: File, playlist_title: &String, playlist_url: &String) -> Result<(), Error> {
    
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
            stdout_print!("Fetching contents of playlist \"{playlist_title}\"");
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
        let ytdl_err_handler = thread::spawn(move || Self::parse_ytdl_stderr(err_reader, tx, procid));
    
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
    
    /// This function assumes that the current directory is the playlist directory and may have undefined behavior otherwise
    /// Manifests are probably soon to be removed.
    pub(super) fn cleanup_old_manifests(&self, old_manifest_count: usize) -> Result<(), io::Error> {
        let files = std::fs::read_dir(current_dir()?)?.collect::<Result<Vec<_>, io::Error>>().unwrap();
        let mut manifests = Vec::new();
        for file in files {
            if Self::file_is_manifest(&Utf16String::from_os_string(file.file_name()).unwrap()) {
                manifests.push(file);
            }
        }


        // nothing to do!
        if manifests.len() < old_manifest_count {
            return Ok(());
        }

        if !self.args.quiet {
            stdout_print!("Cleaning up...");
        }

        manifests.sort_by(|a, b | b.file_name().partial_cmp(&a.file_name()).unwrap());

        while manifests.len() >= 7 {
            let old_manifest = manifests.pop().expect("list should be at least 7 long");
            if self.args.verbose {
                debug_print!("Deleted old manifest {}", old_manifest.file_name().to_str().unwrap());
            }
            std::fs::remove_file(old_manifest.path())?;
        }

        return Ok(());

    }


    pub(super) fn file_is_manifest(filename: &Utf16String) -> bool {
        if (filename.starts_with("playlist".into()) && filename.ends_with(".manifest".into())) || filename == &Utf16String::from_str("playlist-settings.json") {
            true
        } else {
            false
        }
    }


}