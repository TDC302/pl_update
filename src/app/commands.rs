use clap::Subcommand;

#[derive(Subcommand, Debug, Clone)]
pub(crate) enum Commands {
    /// Creates a new directory for a playlist, fetches the playlist manifest
    /// and downloads all associated songs.
    Init { 
        /// The url of the playlist to be downloaded
        playlist_url: String 
    },
    /// Checks playlist for new or removed songs, and downloads/deletes files respectively. 
    /// Requires a valid manifest containing the playlist url.
    Update { 
        /// Optional. If provided the application will use this as the playlist directory.
        playlist_name: Option<String> 
    },
    /// Rebuilds the playlist manifest from the files in the directory. 
    /// Requires a playlist manifest containing at least the playlist url.
    Repair { 
        /// Optional. If provided the application will use this as the playlist directory.
        playlist_name: Option<String> },
    /// Will send the files in the playlist to a connected media device (Android phone, music player) etc..
    /// If device id is not specified, and there is more than one device connected will prompt
    /// user to select device.
    Push { 
        /// Optional. If provided the application will use this as the playlist directory.
        playlist_name: Option<String>,
        /// Optional. The device id to send to.
        device_id: Option<String> 
    }
}