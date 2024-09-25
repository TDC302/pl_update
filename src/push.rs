use std::env;
use std::io;
use std::io::ErrorKind;


use crate::adb;
use crate::Args;

//use adb::AndroidDevice;
use adb::DeviceManager;
use adb::DeviceStatus;


use std::io::Error;

use colored::Colorize;

use crate::pl_update_fatal_error;
//use crate::pl_update_error;
use crate::pl_update_warn;

pub(crate) fn pl_push(options: Args, device_id: Option<String>) -> std::io::Result<()> {
    let mut device_manager;
    let devices;
    let target_device; //Initialize the value to stop the compiler from complaining


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

    let ret = DeviceManager::new("adb");

    
    
    match ret {
        Err(e) => {
            pl_update_fatal_error!(e.kind(), "Adb tool could not be launched, check that it is installed and is accessible (ie. in the system path or working directory)\nReason:{e}");
        },
        Ok(t) => device_manager = t,
    }
    
    pl_update_vprintln!("Found Android Debug Bridge version {}", device_manager.get_version());
    


    devices = device_manager.get_devices()?;

    if devices.is_empty() {
        pl_update_fatal_error!(ErrorKind::NotFound, "No devices were available.");

    }

    if device_id.is_some() {
        let mut found_device = None;

        for device in devices {
            if device.identifier == device_id.clone().unwrap() {
                if found_device.is_none() {
                    found_device = Some(device);
                } else {
                    pl_update_fatal_error!(ErrorKind::AlreadyExists, "Devices {} and {} have duplicate ids.", found_device.unwrap(), device);
                }
            }
        }
        

       
        target_device = found_device.unwrap();
        


    } else {

        if devices.len() == 1 {
            target_device = devices.get(0).expect("device at index 0 should exist").clone();
        } else {


            if options.quiet {
                pl_update_fatal_error!(ErrorKind::WouldBlock, "More than one device was detected, and interactive output was suppressed.");
            }

            pl_update_println!("\nMore than one device was detected, please select from the following list:");
            pl_update_println!("No\t\t\tIdentifier\t\t\tModel");
            let mut i: u8 = 1;
            for device in &devices {
                pl_update_println!("{}\t\t\t{}\t\t\t{}", i, device.identifier, device.model);
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
    
    pl_update_println!("Device {} selected for use.", target_device);



    let status = device_manager.get_device_status(target_device.clone())?;

    match status {
        DeviceStatus::Bootloader => panic!(),
        DeviceStatus::Disconnected => {pl_update_fatal_error!(ErrorKind::ConnectionAborted, "The {} was disconnected before upload could be completed.", target_device);},
        DeviceStatus::Unauthorized => {pl_update_fatal_error!(ErrorKind::PermissionDenied, "This computer was not given debugging access to the {}.", target_device);},
        DeviceStatus::Offline => {
            pl_update_warn!("The {} is offline, attempting to bring online", target_device);
            
            match device_manager.start_offline_device(target_device.clone()) {
                Ok(_) => {},
                Err(e) => {
                    pl_update_fatal_error!(e); 
                },
            }
        }
        DeviceStatus::Online => {}

    }

    let current_dir= env::current_dir()?;
    let current_dir_str = current_dir.to_str().unwrap();

    let ret = device_manager.push_dir(current_dir_str, "/storage/E23F-11FD/music", target_device.clone())?;
    println!("[adb] {}", ret);


    Ok(())

}