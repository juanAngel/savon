#[derive(Debug)]
pub enum Error {
    Wsdl(crate::wsdl::WsdlError),
    Io(std::io::Error),
    Gen(crate::gen::GenError),
    Reqwest(reqwest::Error),
    StringError(String),
    Rpser(crate::rpser::xml::Error),
    Rpc(crate::rpser::RpcError),
    Num(std::num::ParseFloatError),
    Int(std::num::ParseIntError),
    DateTimeParse(chrono::format::ParseError),
    ParseError(String),
}

impl From<crate::wsdl::WsdlError> for Error {
    fn from(e: crate::wsdl::WsdlError) -> Self {
        Error::Wsdl(e)
    }
}

impl From<crate::rpser::xml::Error> for Error {
    fn from(e: crate::rpser::xml::Error) -> Self {
        Error::Rpser(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Reqwest(e)
    }
}

impl From<std::num::ParseFloatError> for Error {
    fn from(e: std::num::ParseFloatError) -> Self {
        Error::Num(e)
    }
}

impl From<chrono::format::ParseError> for Error {
    fn from(e: chrono::format::ParseError) -> Self {
        Error::DateTimeParse(e)
    }
}
