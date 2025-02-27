
use crate::error::InitError;
use crate::playlist_settings::PlaylistSettings;

use core::str;
use std::env::set_current_dir;
use std::fs::create_dir;
use std::fs::read_dir;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::stdout;
use std::io::ErrorKind;
use std::io::Write;
use std::process::Command;

use colored::Colorize;

use super::App;

impl App {
    pub(crate) fn pl_init(&mut self, playlist_url: String) -> Result<(), InitError> {
        macro_rules! pl_update_vprintln {
            ($($x:expr),*) => {
                if self.args.verbose {
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

        macro_rules! pl_update_println {
            ($($x:expr),*) => {
                if !self.args.quiet {
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


        let mut output_args = Vec::new();

        if self.args.verbose {
            output_args.push("--verbose".to_owned());
            output_args.push("--quiet".to_owned());
        }

        self.find_ffmpeg()?;

        output_args.push("--ffmpeg-location".to_owned());
        output_args.push(self.args.ffmpeg_location.to_owned());  

        output_args.push("--simulate".to_owned());
        output_args.push("--flat-playlist".to_owned());
        output_args.push("--lazy-playlist".to_owned());
        output_args.push(playlist_url.clone());
        
        output_args.push("--print".to_owned());
        output_args.push("%(playlist)s".to_owned());
        output_args.push("--playlist-items=1".to_owned());
        
        
        self.find_yt_dl()?;

        let command_name = self.args.yt_dl_location.clone();

        pl_update_vprintln!("Running {} with arguments {:?}", command_name, output_args);

        let ytdl_output = Command::new(&command_name)
                .args(output_args.clone())
                .output()?;



        
        let playlist_name = str::from_utf8(&ytdl_output.stdout).expect("output should be valid utf-8").trim();
        
        stdout().write_all(&ytdl_output.stderr)?;

        
        if playlist_name == "NA" {
            return Err(InitError::InvalidInput)
        } else if playlist_name.contains('\n') {
            panic!();
        }


        match read_dir(playlist_name) {
            Ok(directory) => {
                if directory.count() > 0 {
                    return Err(InitError::AlreadyExists(playlist_name.to_owned()));
                } else {
                    pl_update_vprintln!("Using existing directory \"{}\"", playlist_name);
                }
            },
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    create_dir(playlist_name)?;
                    pl_update_vprintln!("Created directory \"{}\"", playlist_name);
                } else {
                    return Err(e.into());
                }
            }
        }

        
        set_current_dir(playlist_name)?;

        let mut settings = PlaylistSettings::new(playlist_url.clone(), playlist_name.to_string());

        settings.yt_dl_args = self.args.yt_dl_args.clone();
        settings.postprocessor_args = self.args.postproccessor_args.clone();

        settings.write_to_disk("playlist-settings.json")?;

        let manifest = OpenOptions::new().read(true).write(true).create(true).open("playlist.manifest")?;

        pl_update_println!("Fetching contents of playlist \"{playlist_name}\"");

        self.update_manifest(manifest, &playlist_name.to_string(), &playlist_url)?;
        pl_update_println!("Manifest created.");
        
        
        pl_update_println!("Parsing urls from manifest...");
        let songs = Self::parse_manifest(File::open("playlist.manifest")?).unwrap();
        let song_urls: Vec<String> = songs.iter().map(|f| f.url.clone().expect("song should have url")).collect();

        pl_update_println!("Successfully parsed {} urls from manifest.", song_urls.len());
        pl_update_vprintln!("Urls: {:?}", song_urls);

        pl_update_println!("Downloading...");
        self.download(song_urls)?;


        Ok(())
    }
}