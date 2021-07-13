use std::string::FromUtf8Error;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Error {
    Io(std::io::Error),
    Warc(warc::Error),
    UnknownLang(String),
    MetadataConversion(FromUtf8Error),
    Custom(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<warc::Error> for Error {
    fn from(e: warc::Error) -> Error {
        Error::Warc(e)
    }
}
impl From<String> for Error {
    fn from(s: String) -> Error {
        Error::Custom(s)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(e: FromUtf8Error) -> Error {
        Error::MetadataConversion(e)
    }
}
