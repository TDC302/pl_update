use std::env;
use std::env::current_dir;
use std::fs::read_dir;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;


use crate::error::Error;
use crate::FILE_EXT;
use crate::string_parsing::StringExts;

use widestring::WideUtfStr;
use widestring::WideUtfString;
use winmtp::device::device_values::AppIdentifiers;
use winmtp::object::Object;

use super::App;

use colored::Colorize;

use crate::warn_print;
use crate::error_print;
use crate::stdout_print;
use crate::debug_print;

impl App {
    pub(crate) fn push(&mut self, playlist_name: Option<String>, device_id: Option<String>) -> Result<(), Error> {
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
       
        let playlist_folder_name;

        if playlist_name.is_some() {
            playlist_folder_name =  WideUtfString::from(playlist_name.unwrap());
        } else {
            let data: Vec<u16> = current_dir()?.file_name().unwrap().encode_wide().collect();
            playlist_folder_name = WideUtfString::from_vec(data)?;
        }

        let ret = winmtp::Provider::new();

        match ret {
            Err(e) => {
                error_print!("The Windows Media Transfer Protocol provider could not be initialized.");
                return Err(Error::WindowsError(e));
            },
            Ok(t) => mtp_provider = t,
        }

        cnd_print_debug!("MTP Initialized.");


        let devices = mtp_provider.enumerate_devices().unwrap();

        if devices.is_empty() {
            return Err(Error::NoDevices);
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
                return Err(Error::DeviceNotFound(device_id.unwrap()));
            }
            
            


        } else {

            if devices.len() == 1 {
                target_device = devices.get(0).expect("device at index 0 should exist").clone();
            } else {


                if self.args.suppress_interactive {
                    return Err(Error::AmbigousTarget)
                }

                cnd_print_stdout!("\nMore than one device was detected, please select from the following list:");
                cnd_print_stdout!("No\t\t\tID\t\t\tName");
                let mut i: u8 = 1;
                for device in &devices {
                    cnd_print_stdout!("{}\t\t\t{}\t\t\t{}", i, device.device_id(), device.friendly_name());
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
        
        cnd_print_stdout!("Device \"{}\" selected for use.", target_device.friendly_name());



        let device_handle = target_device.open(&identity, true)?;

        let device_root = device_handle.content()?.root()?;    
    
        let children: Vec<Object> = device_root.children()?.collect();

        let root_stor_directory;
        if children.is_empty() {
            return Err(Error::RemoteAccessDenied);
        } else if let Some(obj) = children.iter().find(|&dir| WideUtfString::from_ucstring(dir.name()).unwrap().to_lowercase().contains("sd card".into())) {
            root_stor_directory = obj;
        } else {
            root_stor_directory = children.get(0).unwrap();
        }

        let music_directory;
        if let Some(obj) = root_stor_directory.sub_folders()?.find(|dir| WideUtfString::from_ucstring(dir.name()).unwrap().eq_ignore_case("Music".into())) {
            music_directory = obj;
        } else {
            return Err(Error::RemoteDirectoryNotFound("Music".to_string()))
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

            if Self::file_is_manifest(&file_name) { // skip manifests without warning
                continue;
            }

            if !file_name.ends_with(FILE_EXT.into()) {
                warn_print!("Loose file \"{}\" in local directory.",  file_name);
                continue;
            }
            local_filenames.push(file_name);
        }

        cnd_print_debug!("Found {} local songs: {:?}", local_filenames.len(), local_filenames);

        for mut remote_file in remote_playlist_directory.children()?  {
            let file_name = WideUtfStr::from_ucstr(remote_file.name())?.to_owned();


            if !file_name.ends_with(FILE_EXT.into()) {
                warn_print!("Loose file \"{}\" in remote directory.",  file_name);
                continue;
            }

            let mut found = false;
            for i in 0..local_filenames.len(){
                if *local_filenames.get(i).unwrap() == file_name {
                    cnd_print_debug!("Matched local & remote files \"{file_name}\"");
                    local_filenames.swap_remove(i);
                    found = true;
                    break;
                } 
            }

            if !found {
                cnd_print_stdout!("Deleting removed file \"{file_name}\" from remote.");
                remote_file.delete(true)?;
            }

        }



        cnd_print_stdout!("Pushing {} songs to remote device.", local_filenames.len());

        for local_file in local_filenames {
            cnd_print_stdout!("Pushing file \"{local_file}\" to remote.");
            if let Err(e) = remote_playlist_directory.push_file(Path::new(&local_file.as_ustr().to_os_string()), false) {
                return Err(Error::RemoteFileCreationError(local_file.to_string(), e));
            }
        }


        Ok(())

    }
}
