extern crate chrono;
extern crate os_info;


mod get;
mod push;
mod adb;


use core::str;
use std::fmt::Debug;
use std::env;
use std::io::Error;
use std::process::Command;
use std::str::FromStr;
use std::time::SystemTime;
use chrono::Local;
use colored::Colorize;




#[macro_export]
macro_rules! pl_update_warn {
    ($($x:expr),*) => {
        eprintln!("{} [pl-update] {}", "WARNING:".yellow(),
        format! (
            $(
                $x,
            )*
        )
        )
    };
}

#[macro_export]
macro_rules! pl_update_error {
    ($($x:expr),*) => {
        eprintln!("{} [pl-update] {}", "ERROR:".red().bold(),
        format! (
            $(
                $x,
            )*
        )
        )
    };
}

#[macro_export]
macro_rules! pl_update_fatal_error {
    ($($x:expr),*) => {

        let emsg = format!(
            $(
                $x,
            )*
        );
        eprintln!("\n{} [pl-update] {}\n", "FATAL ERROR:".red().bold(), emsg);
        
        return Err(Error::new(std::io::ErrorKind::Other, emsg));
        
    
    };
}


#[macro_export]
macro_rules! pl_update_ok_exit {
    () => {
        println!("\n\n[pl-update] Operation Completed.");
        std::process::exit(0);
    };
}

#[derive(Debug)]
struct OptionArgs {
    pub verbose: bool,
    pub quiet: bool,
    pub device_id: String,
    pub help: bool,
    pub url: String,
    pub yt_dl_args: Vec<String>,
    pub yt_dl_command: Option<String>,
    pub ffmpeg_command: Option<String>
}

impl OptionArgs {

    pub fn from_string_vec(args: Vec<String>) -> Result<OptionArgs, String> {
        let mut verbose = false;
        let mut quiet = false;
        let mut help = false;
        let mut device = String::from_str("").unwrap();
        let mut url = String::from_str("").unwrap();
        let mut yt_dl_args = Vec::new();
        let mut yt_dl_command = None;
        let mut ffmpeg_command = None;

        for opt in args {
            
            if opt.starts_with("--device") || opt.starts_with("-d") {
                if !opt.starts_with("--device=") && !opt.starts_with("-d=") {
                    Err("Specified the --device option but did not specify device.")?
                } else {
                    device = opt.split_once("=").expect("opt should contain =").1.to_string();
                    
                }
            } else if opt.starts_with("--url") || opt.starts_with("-u") {
                if !opt.starts_with("--url=") && !opt.starts_with("-u=") {
                    Err("Specified the --url option but did not provide url.")?
                } else {
                    url = opt.split_once("=").expect("opt should contain =").1.to_string();
                    
                }
            } else if opt.starts_with("--Xyt-dl") {
                if !opt.starts_with("--Xyt-dl=") {
                    Err("Specified the --Xyt-dl option but did not provide any args to pass.")?
                } else {
                    yt_dl_args.push(opt.split_once("=").expect("opt should contain =").1.to_string());
                    
                }
            } else if opt.starts_with("--ytdl-location") {
                if !opt.starts_with("--ytdl-location=") {
                    Err("Specified the --ytdl-location option but did not specify the location.")?
                } else {
                    yt_dl_command = Some(opt.split_once("=").expect("opt should contain =").1.to_string())
                    
                }
            } else if opt.starts_with("--ffmpeg-location") {
                if !opt.starts_with("--ffmpeg-location=") {
                    Err("Specified the --ffmpeg-location option but did not specify the location.")?
                } else {
                    ffmpeg_command = Some(opt.split_once("=").expect("opt should contain =").1.to_string())
                    
                }
            } else {
                match opt.as_str() {
                    "--verbose" | "-v" => verbose = true,
                    "--quiet" | "-q" => quiet = true,
                    "--help" | "-h" => help = true,
                    _ => Err("No match for: ".to_owned() + &opt)?
                }
            }

        }

        Ok(OptionArgs{verbose, quiet, help, device_id: device, url, yt_dl_args, yt_dl_command, ffmpeg_command})

    }
}

#[derive(Debug)]
enum CommandArg {
    Update,
    Get,
    Init,
    Repair,
    Push

}

impl CommandArg {
    pub fn from_string(s: String) -> Result<Self, ()> {
        use CommandArg::*;
        match s.to_lowercase().as_str() {
            "update" => Ok(Update),
            "get"   => Ok(Get),
            "init" => Ok(Init),
            "repair" => Ok(Repair),
            "push" => Ok(Push),
            _ => Err(())
        }


    }
}


fn main() -> std::io::Result<()> {
    let time: chrono::DateTime<Local> =  SystemTime::now().into();
    let info = os_info::get();

    const SW_MAJOR: u16 = 2;
    const SW_MINOR: u16 = 1;
    const SW_PATCH: u32 = 0;

    
    let command;
    let options;
    
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


    let args: Vec<String> = env::args().collect(); //Proccess the command line args.

    (command, options) = process_args(args)?;

    pl_update_println!("\npl_update version {SW_MAJOR}.{SW_MINOR}.{SW_PATCH}");
    
    if info.architecture().is_some() {
        pl_update_println!("Running on {} {} for {}", info.os_type(), info.version(), info.architecture().unwrap());
    } else {
        pl_update_println!("Running on {} version {}", info.os_type(), info.version());
    }
    pl_update_println!("Started at system time {}\n", time.format("%+"));





    
    pl_update_vprintln!("Command: {:?}, Options: {:?}", command, options);


    match command {
        CommandArg::Get => get::pl_get(options)?,
        CommandArg::Init => todo!(),
        CommandArg::Push => push::pl_push(options)?,
        CommandArg::Repair => todo!(),
        CommandArg::Update => todo!()

    }

 
    pl_update_ok_exit!();

}


fn print_usage(){
    println!("\nUSAGE: pl-update.exe [OPTIONS] URL\n");
    println!("Type \"pl-update.exe --help\" for more detailed information.")
    


}

fn print_help(){
    println!("\nUSAGE: pl-update.exe [OPTIONS] URL\n");

    println!("Options:");
    println!("\t-h, --help\t\t\t\t\t\tPrint this help text and exit");
    println!("\t-v, --verbose\t\t\t\t\t\tPrint various debugging information");
    println!("\t-q, --quiet\t\t\t\t\t\tSuppresses output to warnings and errors only. Not available with --verbose");
    println!("\t--push-to-device\t\t\t\t\tPushes downloaded songs to connected android device.");
    

}


fn process_args(args: Vec<String>) -> std::io::Result<(CommandArg, OptionArgs)> {

    //args.remove(0); // The arg at index 0 is the program name and can be removed.

    let mut command = String::from_str("").unwrap();
    let mut options: Vec<String> = Vec::new();

    

    let mut i = -1;
    for arg in args {
        i += 1;
        if i == 0 {
            continue;
        } else if arg == "" {
            continue;
        } else if arg.starts_with("-") {
            options.push(arg);
            continue;
            
        } else if command == "" {
            command = arg;
            continue;
            
        } else {
            pl_update_fatal_error!("Argument \"{}\" is not recognised.", arg);
        }
        
    }


    let res_opt = OptionArgs::from_string_vec(options);
    let ret_opt;

    match res_opt {
        Ok(val) => ret_opt = val,
        Err(e) => {pl_update_fatal_error!("{}", e);}
    }

    

    if command == "" && ret_opt.help {
        print_help();
        pl_update_ok_exit!();
    } else if command == "" {
        print_usage();
        pl_update_fatal_error!("Command not specified!");
    }
 
    let res_cmd = CommandArg::from_string(command.clone());
    let ret_cmd;

    match res_cmd {
        Ok(val) => ret_cmd = val,
        Err(()) => {pl_update_fatal_error!("Command \"{}\" not recognised.", command);}
    }



    if ret_opt.verbose && ret_opt.quiet {
        print_usage();
        pl_update_fatal_error!("The --quiet option cannot be specified with the --verbose option.");
    }




    Ok((ret_cmd, ret_opt))

}

fn find_yt_dl(verbose: bool, ytdl_command: &String) -> Result<(), Error> {

    //let prog_names = ["yt-dlp", "youtube-dl", "yt-dl"];

   
    let ytdl_check: Result<std::process::Output, Error> = Command::new(ytdl_command.clone()).arg("--version").output();
    
    if ytdl_check.is_ok() {
        let out = &ytdl_check.unwrap().stdout;
        let ver = str::from_utf8(out).unwrap();
        if verbose {
            println!("{} [pl-update] Found {} version {}", "DEBUG:".blue(), ytdl_command, ver);
        }
        return Ok(());
        
    } 
        
    
    

    pl_update_fatal_error!("YT-DL could not be found. Check that it is in the system path or current directory and is accessible.");
    

    //command_name.unwrap().to_string();
}

fn find_ffmpeg(verbose: bool, ffmpeg_command: &String) -> Result<(), Error> {



    let ffmpeg_check: Result<std::process::Output, Error> = Command::new(ffmpeg_command.clone()).arg("--version").output();
        
        if ffmpeg_check.is_ok() {
            let out = &ffmpeg_check.unwrap().stdout;
            let out_data = str::from_utf8(out).unwrap().split(" ").collect::<Vec<_>>();
            let ver = out_data.get(2).unwrap();
            if verbose {
                println!("{} [pl-update] Found {} version {}", "DEBUG:".blue(), ffmpeg_command, ver);
            }
            return Ok(());
            
        } 

    pl_update_fatal_error!("YT-DL could not be found. Check that it is in the system path or current directory and is accessible.");


}

