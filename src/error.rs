use widestring::error::Utf16Error;
use winmtp::{error::{AddFileError, CreateFolderError, MtpError}, WindowsError};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("The Media Transfer Protocol encountered an error: {0}")]
    MtpError(#[from] MtpError),
    #[error("A windows system call failed with error: {0}")]
    WindowsError(#[from] WindowsError),
    #[error("A system call returned a string with invalid or malformed data. Error: {0}")]
    EncodingError(#[from] Utf16Error),
    #[error("System IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("The folder could not be created, error: {0}")]
    FolderCreationError(#[from] CreateFolderError),

    #[error("There are no devices available. Check your device's settings and make sure it's connected.")]
    NoDevices,
    #[error("The device with the specified ID ({0}) could not be found")]
    DeviceNotFound(String),
    #[error("More than one device was detected, and interactive output was supressed.")]
    AmbigousTarget,
    #[error("The target device is not allowing filesystem access, try changing USB permissions or unlocking it.")]
    RemoteAccessDenied,
    #[error("The \"{0}\" directory could not be found.")]
    RemoteDirectoryNotFound(String),
    #[error("The file \"{0}\" could not be sent to the remote deivce, error: {1}")]
    RemoteFileCreationError(String, AddFileError),

   
    #[error("Directory \"{0}\" already exists, and is not empty.")]
    AlreadyExists(String),
    #[error("URL provided was not a playlist, or playlist name was NA (playlist name cannot be NA)")]
    InvalidInput,

    #[error("The playlist settings could not be parsed. Reason: {0}")]
    SettingsParseError(#[from] serde_json::Error),

    #[error("The directory does not have a settings file, either run pl-update with the INIT command, or create a \"playlist-settings.json\" file containing at least your playlist's URL and title")]
    PlaylistUninitialized,

    #[error("The \"{0}\" directory could not be opened. Reason: {1}")]
    DirectoryOpenError(String, std::io::Error),
    #[error("The {0} file could not be opened. Reason: {1}")]
    FileOpenError(String, std::io::Error),
    #[error("The {0} file could not be created. Reason: {1}")]
    FileCreationError(String, std::io::Error)


}

