use colored::Colorize;
use std::fs::remove_file;

use crate::{error::Error, error_print, debug_print, stdout_print};

use super::App;



impl App {


    pub(super) fn update(&mut self) -> Result<(), Error>{

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



        let current_items_list = self.fetch_manifest_local()?;

        let playlist_name = &self.settings.playlist_name;
        let playlist_url = &self.settings.playlist_url;

        cnd_print_stdout!("Found playlist: \"{}\"", playlist_name);

        cnd_print_stdout!("Updating manifest...");

        
        let updated_items_list = self.downloader.fetch_playlist_content(playlist_url, self.args.verbose)?;
        

        let removed_songs: Vec<_> = 
        current_items_list.clone().into_iter().filter(|old_song|
        
            !updated_items_list.contains(old_song)

        ).collect();

        let added_songs: Vec<_> = updated_items_list.into_iter().filter(|new_song|

            !current_items_list.contains(new_song)
        
        ).collect();

        cnd_print_debug!("Detected {} items to download: {:?}", added_songs.len(), added_songs);

        let removed_filenames: Vec<_> = removed_songs.into_iter().map(|f| f.into_filename("mp3".to_owned())).collect();
        let added_urls: Vec<_> = added_songs.into_iter().map(|u| u.url().unwrap()).collect();

        cnd_print_debug!("Detected {} items to remove: {:?}", removed_filenames.len(), removed_filenames);


        let cnt_new_items = added_urls.len();
        if cnt_new_items > 0 {
            cnd_print_stdout!("Downloading {} new items...", added_urls.len());
            let result = self.downloader.download(added_urls, self.args.threads, self.args.progress, self.args.verbose)?;
            cnd_print_stdout!("Res: {result:?}");
            let failures: Vec<_> = result.into_iter().filter_map(
                |(id, res)| 
                if let Err(e) = res {
                    Some((id, e))
                } else {
                    None
                }).collect();

            cnd_print_stdout!("Successfully downloaded {} items.", cnt_new_items - failures.len());
            
            if failures.len() > 0 {
                cnd_print_stdout!("{} items failed to download.", failures.len());
                if self.args.verbose {
                    for (id, err) in failures {
                        debug_print!("ID: {id} failed with error: {err}.");
                    }
                }
            }
        } else {
            cnd_print_stdout!("No items to download.");
        }
        

        if removed_filenames.len() > 0 {
            cnd_print_stdout!("Deleting {} removed items...", removed_filenames.len()); 

            for filename in removed_filenames {
                cnd_print_stdout!("Deleting file {}.", &filename);

                if let Err(e) = remove_file(&filename) {
                    error_print!("Error while deleting file \"{}\"\t {}", &filename, e);
                }
            }
        }  else {
            cnd_print_stdout!("No items to remove.")
        }


        Ok(())
    }
}
