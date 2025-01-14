use std::{fs::{File, OpenOptions}, io::{BufReader, BufWriter}, path::Path};



#[derive(serde::Serialize, serde::Deserialize)]
pub struct PlaylistSettings {
    pub playlist_url: String,
    pub playlist_name: String,
    pub yt_dl_args: Vec<String>,
    pub postprocessor_args: Vec<String>,
    pub backup_manifest_count: Option<usize>
}

impl PlaylistSettings {
    pub fn new(playlist_url: String, playlist_name: String) -> Self {
        PlaylistSettings { playlist_url, playlist_name, yt_dl_args: vec![], postprocessor_args: vec![], backup_manifest_count: Some(7) }
    }

    pub fn write_to_disk<P: AsRef<Path>>(&self, path: P) -> Result<(), std::io::Error> {
        let file = OpenOptions::new().write(true).create(true).truncate(true).open(path)?;
        let buf_writer = BufWriter::new(file);
        serde_json::to_writer_pretty(buf_writer, self).expect("file write should not fail");
        Ok(())
    }

    pub fn from_file(file: File) -> Result<Self, serde_json::Error> {
        let buf_reader = BufReader::new(file);
        serde_json::from_reader(buf_reader)
    }
}