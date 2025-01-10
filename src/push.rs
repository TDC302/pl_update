use std::env;
use std::env::current_dir;
use std::env::set_current_dir;
use std::ffi::OsStr;
use std::fs::read_dir;
use std::io;
use std::io::ErrorKind;
use std::path::Path;


use crate::file_is_manifest;
use crate::Args;
use crate::FILE_EXT;

use winmtp::device::device_values::AppIdentifiers;
use winmtp::object::Object;


use std::io::Error;

use colored::Colorize;

use crate::pl_update_fatal_error;
//use crate::pl_update_error;
use crate::pl_update_warn;

pub(crate) fn pl_push(options: Args, playlist_name: Option<String>, device_id: Option<String>) -> std::io::Result<()> {
    let mtp_provider;
    let target_device;

    let ver_str = env!("CARGO_PKG_VERSION");
    let ver_ints: Vec<u32> =  ver_str.split('.').into_iter().map(|v| u32::from_str_radix(v, 10).expect("Version invalid")).collect();

    let identity = AppIdentifiers {
        app_name: "pl_update".to_owned(),
        app_major: *ver_ints.get(0).unwrap(),
        app_minor: *ver_ints.get(1).unwrap(),
        app_patch: *ver_ints.get(2).unwrap(),

    };



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

    let playlist_folder_name;

    if playlist_name.is_some() {
        playlist_folder_name = playlist_name.unwrap();
        match set_current_dir(playlist_folder_name.clone()) {
            Ok(()) => (),
            Err(err) => {
                pl_update_fatal_error!(err.kind(), "Could not find playlist directory: {}", err);
            }

        }
    } else {
        playlist_folder_name = current_dir()?.file_name().unwrap().to_string_lossy().to_string();
    }

    let ret = winmtp::Provider::new();

    match ret {
        Err(e) => {
            pl_update_fatal_error!(ErrorKind::Other, "The Windows Media Transfer Protocol provider could not be initialized.\nReason:{e}");
        },
        Ok(t) => mtp_provider = t,
    }

    pl_update_vprintln!("MTP Initialized.");


    let devices = mtp_provider.enumerate_devices().unwrap();

    if devices.is_empty() {
        pl_update_fatal_error!(ErrorKind::NotFound, "No devices were available.");

    }

    if device_id.is_some() {
        let mut found_device = None;

        for device in devices {
            if device.device_id() == device_id.clone().unwrap() {
                if found_device.is_none() {
                    found_device = Some(device);
                } else {
                    pl_update_fatal_error!(ErrorKind::AlreadyExists, "Devices {} and {} have duplicate ids.", found_device.unwrap().friendly_name(), device.friendly_name());
                }
            }
        }
        

        if found_device.is_some() {
            target_device = found_device.unwrap();
        } else {
            pl_update_fatal_error!(ErrorKind::NotFound, "The device with id {} could not be found.", device_id.unwrap());
        }
        
        


    } else {

        if devices.len() == 1 {
            target_device = devices.get(0).expect("device at index 0 should exist").clone();
        } else {


            if options.quiet {
                pl_update_fatal_error!(ErrorKind::WouldBlock, "More than one device was detected, and interactive output was suppressed.");
            }

            pl_update_println!("\nMore than one device was detected, please select from the following list:");
            pl_update_println!("No\t\t\tID\t\t\tName");
            let mut i: u8 = 1;
            for device in &devices {
                pl_update_println!("{}\t\t\t{}\t\t\t{}", i, device.device_id(), device.friendly_name());
                i += 1;
            }
            


            loop {
                print!("Enter a device number to select: ");
                let mut buffer = String::new();
                io::stdin().read_line(&mut buffer)?;

                buffer = buffer.trim().to_string();

                let selection = buffer.parse::<usize>();
                match selection {
                    Ok(_) => {
                        let num = selection.unwrap();
                        if num > devices.len()  {
                            continue;
                        } else {
                            target_device = devices.get(num - 1).expect("index should not be out of bounds").clone();
                            break; 
                        }

                    },
                    Err(_) => continue,
                }
            }


        }

    } 
    
    pl_update_println!("Device \"{}\" selected for use.", target_device.friendly_name());



    let device_handle = target_device.open(&identity, true)?;

    let device_content = device_handle.content().unwrap();    

    let device_root = device_content.root()?;

   
    let children: Vec<Object> = device_root.children()?.collect();

    let root_stor_directory;
    if children.is_empty() {
        pl_update_fatal_error!(ErrorKind::PermissionDenied, "The target device is not allowing filesystem access, try changing USB permissions or unlocking it.");
    } else if let Some(obj) = children.iter().find(|&dir| dir.name().to_string_lossy().to_ascii_lowercase().contains("sd card")) {
        root_stor_directory = obj;
    } else {
        root_stor_directory = children.get(0).unwrap();
    }

    let music_directory;
    if let Some(obj) = root_stor_directory.sub_folders()?.find(| dir| dir.name().to_string_lossy().eq_ignore_ascii_case("Music")) {
        music_directory = obj;
    } else {
        pl_update_fatal_error!(ErrorKind::NotFound, "The music folder could not be found.");
    }
    
    let remote_playlist_directory;
    if let Some(obj) = music_directory.sub_folders()?.find(| dir | dir.name().to_string_lossy().eq(&playlist_folder_name)) {
        remote_playlist_directory = obj;
    } else {
        let id = music_directory.create_subfolder(OsStr::new(&playlist_folder_name)).unwrap();
        remote_playlist_directory = music_directory.sub_folders()?.find(|dir| dir.id() == id).expect("created folder should exist");
    }

    let local_playlist_directory = read_dir(".")?.collect::<Result<Vec<_>, io::Error>>().unwrap();

    let mut local_filenames = Vec::new();
    for local_file in local_playlist_directory  {
        let file_name = local_file.file_name().into_string().unwrap();

        if file_is_manifest(&file_name) { // skip manifests without warning
            continue;
        }

        if !file_name.ends_with(FILE_EXT) {
            pl_update_warn!("Loose file \"{}\" in local directory.",  file_name);
            continue;
        }
        local_filenames.push(file_name);
    }

    pl_update_vprintln!("Found {} local songs: {:?}", local_filenames.len(), local_filenames);

    for mut remote_file in remote_playlist_directory.children()?  {
        let file_name = remote_file.name().to_string_lossy();

        if file_is_manifest(&file_name) { // skip manifests without warning
            continue;
        }

        if !file_name.ends_with(FILE_EXT) {
            pl_update_warn!("Loose file \"{}\" in remote directory.",  file_name);
            continue;
        }

        let mut found = false;
        for i in 0..local_filenames.len(){
            if *local_filenames.get(i).unwrap() == file_name {
                pl_update_vprintln!("Matched local & remote files \"{file_name}\"");
                local_filenames.swap_remove(i);
                found = true;
                break;
            } 
        }

        if !found {
            pl_update_vprintln!("Failed match on remote \"{file_name}\", deleting...");
            remote_file.delete(true)?;
        }

    }

    pl_update_println!("Pushing {} songs to device", local_filenames.len());

    for local_file in local_filenames {
        pl_update_vprintln!("Pushing {local_file}");
        remote_playlist_directory.push_file(Path::new(&local_file), false).unwrap();
    }


    Ok(())

}