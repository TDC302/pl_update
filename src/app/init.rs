
use crate::error::Error;
use crate::playlist_settings::PlaylistSettings;

use core::str;
use std::env::set_current_dir;
use std::fs::create_dir;
use std::fs::read_dir;
use std::io::stdout;
use std::io::ErrorKind;
use std::io::Write;
use std::process::Command;


use colored::Colorize;

use crate::{debug_print, stdout_print};

use super::App;

impl App {
    pub(crate) fn init(&mut self, playlist_url: String) -> Result<(), Error> {
        
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


        let mut output_args = Vec::new();

        if self.args.verbose {
            output_args.push("--verbose".to_owned());
        } else if self.args.quiet {
            output_args.push("--quiet".to_owned());
        }


        output_args.push("--simulate".to_owned());
        output_args.push("--flat-playlist".to_owned());
        output_args.push("--lazy-playlist".to_owned());
        output_args.push(playlist_url.clone());
        
        output_args.push("--print".to_owned());
        output_args.push("%(playlist)s".to_owned());
        output_args.push("--playlist-items=1".to_owned());
        
        
        self.find_yt_dl()?;

        let command_name = self.args.yt_dl_location.clone();

        cnd_print_debug!("Running {} with arguments {:?}", command_name, output_args);

        let ytdl_output = Command::new(&command_name)
                .args(output_args.clone())
                .output()?;



        
        let playlist_name = str::from_utf8(&ytdl_output.stdout).expect("output should be valid utf-8").trim();
        
        stdout().write_all(&ytdl_output.stderr)?;

        
        if playlist_name == "NA" {
            return Err(Error::InvalidInput)
        } else if playlist_name.contains('\n') {
            panic!();
        }


        match read_dir(playlist_name) {
            Ok(directory) => {
                if directory.count() > 0 {
                    return Err(Error::AlreadyExists(playlist_name.to_owned()));
                } else {
                    cnd_print_debug!("Using existing directory \"{}\"", playlist_name);
                }
            },
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    create_dir(playlist_name)?;
                    cnd_print_debug!("Created directory \"{}\"", playlist_name);
                } else {
                    return Err(e.into());
                }
            }
        }

        
        set_current_dir(playlist_name)?;
        create_dir(".playlist")?;

        let mut settings = PlaylistSettings::new(playlist_url.clone(), playlist_name.to_string());

        settings.yt_dl_args = self.args.yt_dl_args.clone();
        settings.postprocessor_args = self.args.postproccessor_args.clone();

        settings.write_to_disk()?;

        let songs = self.fetch_manifest_url(&playlist_name.to_string(), &playlist_url)?;
        

        let song_urls: Vec<String> = songs.iter().map(|f| f.url.clone().expect("song should have url")).collect();

        cnd_print_stdout!("Successfully parsed {} urls from manifest.", song_urls.len());
        cnd_print_debug!("Urls: {:?}", song_urls);

        cnd_print_stdout!("Downloading...");
        self.download(song_urls)?;


        Ok(())
    }
}