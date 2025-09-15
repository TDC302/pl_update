use std::{fs::{exists, File, OpenOptions}, io::{BufReader, BufWriter}};

use crate::error::Error;

/// The filename to use for playlist settings files.
pub(crate) const PLAYLIST_SETTINGS_PATH: &str = ".playlist/settings.json";

/// The old playlist filename for compatibility
pub(crate)  const PLAYLIST_SETTINGS_NAME_COMPAT: &str = "playlist-settings.json";


#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct PlaylistSettings {
    pub playlist_url: String,
    pub playlist_name: String,
    pub yt_dl_args: Vec<String>,
    pub postprocessor_args: Vec<String>,
    pub backup_manifest_count: Option<usize>,

    #[serde(skip)]
    uses_old_format: bool,
}

impl PlaylistSettings {
    pub fn new(playlist_url: String, playlist_name: String) -> Self {
        PlaylistSettings { playlist_url, playlist_name, yt_dl_args: vec![], postprocessor_args: vec![], backup_manifest_count: Some(7), uses_old_format: false }
    }

    pub fn write_to_disk(&self) -> Result<(), std::io::Error> {
        let file = OpenOptions::new().write(true).create(true).truncate(true).open(PLAYLIST_SETTINGS_PATH)?;
        let buf_writer = BufWriter::new(file);
        serde_json::to_writer_pretty(buf_writer, self).expect("file write should not fail");
        Ok(())
    }
    
    pub fn format_is_depreciated(&self) -> bool {
        self.uses_old_format
    }

    pub fn try_open() -> Result<Self, Error> {
        let mut old_path = false;
        let tgt_path = 
        if exists(PLAYLIST_SETTINGS_PATH)? {
            PLAYLIST_SETTINGS_PATH
        } else if exists(PLAYLIST_SETTINGS_NAME_COMPAT)? {
            old_path = true;
            PLAYLIST_SETTINGS_NAME_COMPAT
        } else {
            return Err(Error::PlaylistUninitialized);
        };

        let mut ret: Self = serde_json::from_reader(BufReader::new(File::open(tgt_path)?))?;
        ret.uses_old_format = old_path;
        
        Ok(ret)

    }
}