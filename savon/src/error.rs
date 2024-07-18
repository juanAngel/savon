#[derive(Debug)]
pub enum Error {
    Wsdl(crate::wsdl::WsdlError),
    Io(std::io::Error),
    Gen(crate::gen::GenError),
    Reqwest(reqwest::Error),
    Rpser(crate::rpser::xml::Error),
    Num(std::num::ParseFloatError),
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
