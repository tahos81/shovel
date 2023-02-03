use thiserror::Error;

#[derive(Debug, Error)]
pub enum UploadError {
    #[error("File is larger than max upload size")]
    ExceedsMaxSize(),

    #[error("Bad file size")]
    BadSize(),
}
