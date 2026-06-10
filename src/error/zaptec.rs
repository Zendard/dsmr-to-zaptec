use core::{num::ParseIntError, str::Utf8Error};

#[derive(Debug, defmt::Format)]
pub enum ZaptecError {
    TokenRequest(reqwless::Error),
    TokenResponse(TokenResponseError),
}

#[derive(Debug, defmt::Format)]
pub enum TokenResponseError {
    NotAString,
    NoTokenLine,
    NoExpiresLine,
    MalformedExpiresLine,
}

impl From<TokenResponseError> for ZaptecError {
    fn from(e: TokenResponseError) -> Self {
        Self::TokenResponse(e)
    }
}

impl From<ParseIntError> for ZaptecError {
    fn from(_: ParseIntError) -> Self {
        Self::TokenResponse(TokenResponseError::MalformedExpiresLine)
    }
}

impl From<Utf8Error> for ZaptecError {
    fn from(_: Utf8Error) -> Self {
        Self::TokenResponse(TokenResponseError::NotAString)
    }
}

impl From<reqwless::Error> for ZaptecError {
    fn from(e: reqwless::Error) -> Self {
        Self::TokenRequest(e)
    }
}
