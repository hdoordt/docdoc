use std::{fmt, io, path::PathBuf};

#[derive(Debug)]
pub enum Error {
    ImportCycle(PathBuf),
    Io(io::Error),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ImportCycle(path) => write!(
                f,
                "File at {} was imported multiple times",
                path.to_string_lossy()
            ),
            Error::Io(e) => write!(f, "IO Error: {e}"),
        }
    }
}

impl std::error::Error for Error {}
