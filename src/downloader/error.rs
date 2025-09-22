


#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("The downloader '{0}' could not be found. Check that it is installed and acessible.")]
    InitializationError(String),
    
    #[error("Error while parsing {0}: {1}")]
    ParserError(String, String),

    #[error("The specified playlist could not be found.")]
    PlaylistDoesNotExist,
    #[error("Downloader error: {0}")]
    DowloaderError(String),

    #[error("System IO error: {0}")]
    IoError(#[from] std::io::Error),
}