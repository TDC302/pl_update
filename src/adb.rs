

use std::{fs, io::{self, Error, ErrorKind}, process::{Command, Stdio}, thread::sleep, time::Duration};
use std::fmt::Debug;


#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DeviceStatus {
    Online,
    Offline,
    Bootloader,
    Unauthorized,
    Disconnected
} 


impl DeviceStatus {
    pub fn as_str(&self) -> &'static str {
        use DeviceStatus::*;
        
        match *self {
            Online => "ready for command",
            Offline => "offline",
            Bootloader => "in bootloader",
            Unauthorized => "unauthorized",
            Disconnected => "no longer connected"
        }


    }
}

impl std::fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}






#[derive(Debug)]
pub struct DeviceManager {
    adb_command: String,
    adb_version: Vec<i32>,
    devices: Vec<AndroidDevice>
}

#[derive(Debug, Clone, Eq)]
pub(crate) struct AndroidDevice {
    pub identifier: String,
    pub model: String,
    transport_id: u32
}

impl std::fmt::Display for AndroidDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{} with ID: {}", self.model, self.identifier)
    }
}   


impl PartialEq for AndroidDevice {
    fn eq(&self, other: &Self) -> bool {
        self.identifier == other.identifier && self.model == other.model //This implementation of PartialEq does not check transport_id as it is subject to change
    }
}



impl AndroidDevice {

    fn new(identifier: String, model: String, transport_id: u32) -> AndroidDevice {

        return AndroidDevice {identifier, model, transport_id} ;
    }


    pub fn is_decoy(&self) -> bool{
        if self.transport_id == 99 {
            true
        } else {
            false
        }

    }


}

impl DeviceManager {


    pub fn new(command: &str) -> io::Result<DeviceManager> {

        let adb_inst = Command::new(command).arg("--version").stdout(Stdio::piped()).spawn()?;
        let ver_no: Vec<i32>;
        
        let mut output = Self::dump_stdout(adb_inst)?;
        

   

        output = output.split_off(output.find("Android Debug Bridge version ").expect("adb should have version") + "Android Debug Bridge version ".len());


        output.truncate(output.find("\r\n").expect("buf should contain newline"));            
        
        

        ver_no = output.split(".").map(|num| num.parse::<i32>().unwrap()).collect();





        Ok(DeviceManager{adb_command: command.to_string(),  adb_version: ver_no, devices: Vec::new()})

    }


    pub fn get_devices(&mut self) -> io::Result<Vec<AndroidDevice>> {
        
        self.refresh_devices()?;

        let mut output_devices = Vec::new();

        self.devices.clone_into(&mut output_devices);

        Ok(output_devices)
    }
    



    fn refresh_devices(&mut self) -> io::Result<()> {
        
        let adb_inst = Command::new(&self.adb_command).args(["devices","-l"]).stdout(Stdio::piped()).spawn()?;
        let mut devices: Vec<AndroidDevice> = Vec::new();

        let output = Self::dump_stdout(adb_inst)?;


        let device_list = output.split("\r\n");
 

        for list_entry in device_list {
            if list_entry.starts_with("List of devices attached") || list_entry.is_empty() {
                continue;
            } 

            let identifier = list_entry.split_once(' ').unwrap().0.to_string();
            let model = list_entry.split_once("model:").unwrap().1.split_once(' ').unwrap().0.to_string();
            let transport_id = list_entry.split_once("transport_id:").unwrap().1.parse::<u32>().expect("could not parse transport id");

            devices.push(AndroidDevice::new(identifier, model, transport_id));


        }



        self.devices.clear();
        self.devices.append(&mut devices);


        Ok(())
    }


    pub fn push_dir(&mut self, source_dir: &str, dest_dir: &str, target_device: AndroidDevice) -> io::Result<String> {
    

        

        if !Self::path_exists(source_dir) {
            return Err(Error::new(ErrorKind::NotFound, format!("Could not find \"{}\"", source_dir)));
        }

 
       
        let transport_id;

        
        let status = self.get_device_status(target_device.clone())?;


        match status {
            DeviceStatus::Disconnected => return Err(Error::new(ErrorKind::NotConnected, format!("The {} is no longer connected.", target_device))),
            DeviceStatus::Offline => return Err(Error::new(ErrorKind::AddrNotAvailable, format!("The {} is offline.", target_device))),
            DeviceStatus::Bootloader => return Err(Error::new(ErrorKind::Unsupported, format!("The {} is in bootloader and cannot be used.", target_device))),
            DeviceStatus::Unauthorized => return Err(Error::new(ErrorKind::PermissionDenied, format!("The {} has not granted this device adb access.", target_device))),
            DeviceStatus::Online => {}
        }


        transport_id = self.get_transport_id(target_device.clone());




        let adb_inst = Command::new(&self.adb_command).args(["-t", transport_id.to_string().as_str(), "shell", "cd", dest_dir]).stdout(Stdio::piped()).spawn()?;
        let mut buffer = Self::dump_stdout(adb_inst)?;
        

        buffer = buffer.trim().to_string();



        if buffer.ends_with("No such file or directory") || dest_dir.is_empty(){
            return Err(Error::new(ErrorKind::NotFound, format!("Could not find \"{}\"", dest_dir)));
        } 
          

        let adb_inst = Command::new(&self.adb_command).args(["-t", transport_id.to_string().as_str(), "push", "--sync", source_dir, dest_dir]).stdout(Stdio::piped()).spawn()?;
        let mut buffer = Self::dump_stdout(adb_inst)?;

        buffer = buffer.trim().to_string();
        
        
        let last;
        if buffer.contains("\r\n") {
            
            last = buffer.split("\r\n").last().expect("last line of buffer should not be empty").to_owned(); 

            
        } else {
            last = buffer;
        }


        Ok(last)

    }


    pub fn start_offline_device(&mut self, target_device: AndroidDevice) -> io::Result<()> {
        
        
        
        let mut status = self.get_device_status(target_device.clone())?;

        if status != DeviceStatus::Offline {
           return Err(Error::other(format!("Expected device to be offline, but device was {}", status)));
        }

        let transport_id = self.get_transport_id(target_device.clone());

        let mut adb_inst = Command::new(&self.adb_command).args(["-t", transport_id.to_string().as_str(), "reconnect"]).stdout(Stdio::piped()).spawn()?;
        //let buffer = Self::dump_stdout(adb_inst)?;
        adb_inst.wait()?;

        

        let mut count = 0;

        loop {
            status = self.get_device_status(target_device.clone())?;
        
            match status {
                DeviceStatus::Online => return Ok(()),
                DeviceStatus::Bootloader => return Err(Error::new(ErrorKind::Unsupported, format!("The {} reconnected in bootloader... somehow???", target_device))),
                DeviceStatus::Disconnected => return Err(Error::new(ErrorKind::NotConnected, format!("The {} is no longer connected.", target_device))),
                DeviceStatus::Unauthorized => return Err(Error::new(ErrorKind::NotConnected, format!("The {} has not granted this device adb access.", target_device))),
                DeviceStatus::Offline => {
                    sleep(Duration::from_millis(200));
                    if count >= 3 {
                        return Err(Error::new(ErrorKind::AddrNotAvailable, format!("The {} could not be brought online.", target_device)));
                    }

                }

            }
            count += 1;
        
        }



        
    }

    pub fn get_version(&self) -> String {
        let mut ret = String::new();
        for i in &self.adb_version {
            ret.push_str(i.to_string().as_str());
            ret.push('.');
        }
        ret.pop();
        ret
    }

    fn get_transport_id(&self, input_device: AndroidDevice) -> u32 {
        
        for device in &self.devices {
            if *device == input_device {
                return device.transport_id;
            
            }

        }
        return 99;
    }

    pub fn get_device_status(&mut self, input_device: AndroidDevice) -> io::Result<DeviceStatus> {

        self.refresh_devices()?;

        if input_device.is_decoy() {
            return Err(Error::new(ErrorKind::NotFound, format!("Device does not exist!")));
        }

        if !self.devices.contains(&input_device) {
            return Ok(DeviceStatus::Disconnected);
        }

        
        

        let transport_id = Self::get_transport_id(&self, input_device.clone());

        let adb_inst = Command::new(&self.adb_command).args(["-t", transport_id.to_string().as_str(), "get-state"]).stdout(Stdio::piped()).spawn()?;

        let mut buffer = Self::dump_stdout(adb_inst)?;
        
        buffer = buffer.trim().to_string();

        match buffer.as_str() {
            "device" => Ok(DeviceStatus::Online),
            "offline" => Ok(DeviceStatus::Offline),
            "bootloader" => Ok(DeviceStatus::Bootloader),
            "unauthorized" => Ok(DeviceStatus::Unauthorized),
            &_ => Err(Error::new(ErrorKind::UnexpectedEof, format!("Could not get status for {}", input_device)))
        }

    }

  

    fn path_exists(path: &str) -> bool {
        let metadata = fs::metadata(path);
        
        if metadata.is_ok() {
            true 
        } else {
            false
        }

    }


    fn dump_stdout(process: std::process::Child) -> io::Result<String> {
        let ret = process.wait_with_output()?;
        Ok(String::from_utf8(ret.stdout).expect("string should contain utf-8"))
    }
    

}



