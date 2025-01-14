
use crate::error::InitError;
use crate::playlist_settings::PlaylistSettings;
use crate::update_manifest;
use crate::Args;

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

use crate::download;
use crate::find_yt_dl;

use crate::parse_manifest;




pub(crate) fn pl_init(options: Args, playlist_url: String) -> Result<(), InitError> {
    macro_rules! pl_update_vprintln {
        ($($x:expr),*) => {
            if options.verbose {
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
            if !options.quiet {
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

    if options.verbose {
        output_args.push("--verbose".to_owned());
        output_args.push("--quiet".to_owned());
    }

    if options.ffmpeg_location.is_some() {
        output_args.push("--ffmpeg-location".to_owned());
        output_args.push(options.ffmpeg_location.to_owned().unwrap());
    
    }

    output_args.push("--simulate".to_owned());
    output_args.push("--flat-playlist".to_owned());
    output_args.push("--lazy-playlist".to_owned());
    output_args.push(playlist_url.clone());
    
    output_args.push("--print".to_owned());
    output_args.push("%(playlist)s".to_owned());
    output_args.push("--playlist-items=1".to_owned());
    
    
    let command_name = options.yt_dl_location.clone();
    find_yt_dl(options.verbose, &command_name)?;

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

    settings.yt_dl_args = options.yt_dl_args.clone();
    settings.postprocessor_args = options.postproccessor_args.clone();

    settings.write_to_disk("playlist-settings.json")?;

    let manifest = OpenOptions::new().read(true).write(true).create(true).open("playlist.manifest")?;

    pl_update_println!("Fetching contents of playlist \"{playlist_name}\"");

    update_manifest(manifest, playlist_name.to_string(), &playlist_url, &options)?;
    pl_update_println!("Manifest created.");
    
    
    pl_update_println!("Parsing urls from manifest...");
    let songs = parse_manifest(File::open("playlist.manifest")?).unwrap();
    let song_urls: Vec<String> = songs.iter().map(|f| f.url.clone().expect("song should have url")).collect();

    pl_update_println!("Successfully parsed {} urls from manifest.", song_urls.len());
    pl_update_vprintln!("Urls: {:?}", song_urls);

    pl_update_println!("Downloading...");
    download(song_urls, &options)?;


    Ok(())
}