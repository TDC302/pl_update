use std::env;
use std::env::current_dir;
use std::env::set_current_dir;
use std::fs::read_dir;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;


use crate::error::PushError;
use crate::file_is_manifest;
use crate::pl_update_error;
use crate::Args;
use crate::FILE_EXT;
use crate::string_parsing::StringExts;

use widestring::WideUtfStr;
use widestring::WideUtfString;
use winmtp::device::device_values::AppIdentifiers;
use winmtp::object::Object;


use colored::Colorize;

use crate::pl_update_warn;

pub(crate) fn pl_push(options: Args, playlist_name: Option<String>, device_id: Option<String>) -> Result<(), PushError> {
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

    let playlist_folder_name: WideUtfString;

    if playlist_name.is_some() {
        playlist_folder_name =  WideUtfString::from(playlist_name.unwrap());
        match set_current_dir(playlist_folder_name.to_string()) {
            Ok(()) => (),
            Err(_) => {
                return Err(PushError::NotFound(playlist_folder_name.to_string()))
            }
            
        }
    } else {
        let data: Vec<u16> = current_dir()?.file_name().unwrap().encode_wide().collect();
        playlist_folder_name = WideUtfString::from_vec(data)?;
    }

    let ret = winmtp::Provider::new();

    match ret {
        Err(e) => {
            pl_update_error!("The Windows Media Transfer Protocol provider could not be initialized.");
            return Err(PushError::WindowsError(e));
        },
        Ok(t) => mtp_provider = t,
    }

    pl_update_vprintln!("MTP Initialized.");


    let devices = mtp_provider.enumerate_devices().unwrap();

    if devices.is_empty() {
        return Err(PushError::NoDevices);
    }

    if device_id.is_some() {
        let mut found_device = None;

        for device in devices {
            if device.device_id() == device_id.clone().unwrap() {
                if found_device.is_none() {
                    found_device = Some(device);
                    break;
                } 
            }
        }
        

        if found_device.is_some() {
            target_device = found_device.unwrap();
        } else {
            return Err(PushError::DeviceNotFound(device_id.unwrap()));
        }
        
        


    } else {

        if devices.len() == 1 {
            target_device = devices.get(0).expect("device at index 0 should exist").clone();
        } else {


            if options.suppress_interactive {
                return Err(PushError::AmbigousTarget)
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
                            println!("Invald.");
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

    let device_root = device_handle.content()?.root()?;    
   
    let children: Vec<Object> = device_root.children()?.collect();

    let root_stor_directory;
    if children.is_empty() {
        return Err(PushError::AccessDenied);
    } else if let Some(obj) = children.iter().find(|&dir| WideUtfString::from_ucstring(dir.name()).unwrap().to_lowercase().contains("sd card".into())) {
        root_stor_directory = obj;
    } else {
        root_stor_directory = children.get(0).unwrap();
    }

    let music_directory;
    if let Some(obj) = root_stor_directory.sub_folders()?.find(|dir| WideUtfString::from_ucstring(dir.name()).unwrap().eq_ignore_case("Music".into())) {
        music_directory = obj;
    } else {
        return Err(PushError::NotFound("Music".to_string()))
    }
    
    let remote_playlist_directory;
    if let Some(obj) = music_directory.sub_folders()?.find(| dir | dir.name().to_string_lossy().eq(&playlist_folder_name)) {
        remote_playlist_directory = obj;
    } else {
        match music_directory.create_subfolder(&playlist_folder_name.as_ustr().to_os_string()) {
            Ok(id) => remote_playlist_directory = music_directory.sub_folders()?.find(|dir| dir.id() == id).expect("created folder should exist"),
            Err(e) => {return Err(e.into());}
        }
    }

    let local_playlist_directory = read_dir(".")?.collect::<Result<Vec<_>, io::Error>>().unwrap();

    let mut local_filenames = Vec::new();
    for local_file in local_playlist_directory  {
        let file_name = WideUtfString::from_os_string(local_file.file_name())?;

        if file_is_manifest(&file_name) { // skip manifests without warning
            continue;
        }

        if !file_name.ends_with(FILE_EXT.into()) {
            pl_update_warn!("Loose file \"{}\" in local directory.",  file_name);
            continue;
        }
        local_filenames.push(file_name);
    }

    pl_update_vprintln!("Found {} local songs: {:?}", local_filenames.len(), local_filenames);

    for mut remote_file in remote_playlist_directory.children()?  {
        let file_name = WideUtfStr::from_ucstr(remote_file.name())?.to_owned();


        if !file_name.ends_with(FILE_EXT.into()) {
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
            pl_update_println!("Deleting removed file \"{file_name}\" from remote.");
            remote_file.delete(true)?;
        }

    }



    pl_update_println!("Pushing {} songs to remote device.", local_filenames.len());

    for local_file in local_filenames {
        pl_update_println!("Pushing file \"{local_file}\" to remote.");
        if let Err(e) = remote_playlist_directory.push_file(Path::new(&local_file.as_ustr().to_os_string()), false) {
            return Err(PushError::FileCreationError(local_file.to_string(), e));
        }
    }


    Ok(())

}

