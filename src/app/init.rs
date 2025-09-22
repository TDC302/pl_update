
use crate::error::Error;
use crate::playlist_settings::PlaylistSettings;

use std::env::set_current_dir;
use std::fs::create_dir;
use std::fs::read_dir;
use std::io::ErrorKind;


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

        let playlist_name = self.downloader.fetch_playlist_name(&playlist_url, self.args.verbose)?;


        match read_dir(&playlist_name) {
            Ok(directory) => {
                if directory.count() > 0 {
                    return Err(Error::AlreadyExists(playlist_name));
                } else {
                    cnd_print_debug!("Using existing directory \"{}\"", playlist_name);
                }
            },
            Err(e) => {
                if e.kind() == ErrorKind::NotFound {
                    create_dir(&playlist_name)?;
                    cnd_print_debug!("Created directory \"{}\"", playlist_name);
                } else {
                    return Err(e.into());
                }
            }
        }

        
        set_current_dir(&playlist_name)?;
        create_dir(".playlist")?;

        let mut settings = PlaylistSettings::new(playlist_url.clone(), playlist_name.to_string());

        settings.yt_dl_args = self.args.yt_dl_args.clone();
        settings.postprocessor_args = self.args.postproccessor_args.clone();

        settings.write_to_disk()?;

        let songs = self.downloader.fetch_playlist_content(&playlist_url, self.args.verbose)?;
        

        let song_urls: Vec<String> = songs.iter().map(|f| f.url.clone().expect("song should have url")).collect();

        let cnt_items = song_urls.len();
        cnd_print_stdout!("Successfully parsed {} urls from manifest.", cnt_items);
        cnd_print_debug!("Urls: {:?}", song_urls);

        cnd_print_stdout!("Downloading...");
        
        
        let failures: Vec<_> = self.downloader.download(song_urls, self.args.threads, self.args.progress, self.args.verbose)?
                .into_iter().filter_map(|(id, res)| 
                if let Err(e) = res {
                    Some((id, e))
                } else {
                    None
                }).collect();

        cnd_print_stdout!("Successfully downloaded {} items.", cnt_items - failures.len());

        if failures.len() > 0 {
            cnd_print_stdout!("{} items failed to download.", failures.len());
            if self.args.verbose {
                for (id, err) in failures {
                    debug_print!("ID: {id} failed with error: {err}.");
                }
            }
        };


        Ok(())
    }
}