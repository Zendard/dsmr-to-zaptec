use crate::{
    error::zaptec::{TokenResponseError, ZaptecError},
    mk_static,
    wifi::{ReqwlessClient, ReqwlessConnection},
};
use const_format::formatcp;
use defmt::{info, warn};
use embassy_time::{Duration, Instant};
use reqwless::{
    client::HttpRequestHandle,
    request::{Method, RequestBuilder},
};

const ZAPTEC_URL: &str = "https://api.zaptec.com";
const ZAPTEC_HOST: &str = "api.zaptec.com";

#[derive(defmt::Format)]
pub struct ZaptecSettings {
    pub charging_power: f64,
}

pub struct ZaptecClient<'a> {
    https_client: ReqwlessClient,
    token: ZaptecToken<'a>,
}

#[derive(defmt::Format)]
pub struct ZaptecToken<'a> {
    access_token: &'a str,
    expiration: Instant,
}

impl<'a, 'b> ZaptecClient<'a> {
    pub async fn new(mut https_client: ReqwlessClient) -> Result<Self, ZaptecError> {
        let (token, expires_in) = Self::request_token(&mut https_client).await?;

        let mut buf = [0u8; 4096];
        let buf_ref = mk_static!([u8; 4096], buf);
        let token_bytes = token.as_bytes();
        buf[..token_bytes.len()].copy_from_slice(token_bytes);
        let token_copy = str::from_utf8(&buf_ref[..token_bytes.len()]).unwrap();

        Ok(Self {
            https_client,
            token: ZaptecToken::new(token_copy, expires_in),
        })
    }

    pub fn apply_settings(&self, _settings: ZaptecSettings) -> Result<(), ZaptecError> {
        Ok(())
    }

    async fn request_token(
        https_client: &'b mut ReqwlessClient,
    ) -> Result<(&'b str, u64), ZaptecError> {
        let mut request = Self::make_token_request(https_client).await?;
        let rx_buf = [0u8; 4096];
        let rx_buf_ref = mk_static!([u8; 4096], rx_buf);
        let response = request.send(rx_buf_ref).await?;
        let response_body = response.body().read_to_end().await?;
        let response_body_ref = mk_static!(&[u8], response_body);

        let (token, expires_in) = Self::parse_token_response(str::from_utf8(response_body_ref)?)?;

        Ok((token, expires_in))
    }

    fn parse_token_response(response_body: &'b str) -> Result<(&'b str, u64), ZaptecError> {
        info!("Token response body: {}", response_body);
        let mut response_lines = response_body.lines();
        response_lines.next();
        let token_line = response_lines
            .next()
            .ok_or(TokenResponseError::NoTokenLine)?;

        if !token_line.contains("access_token") {
            warn!("No acces_token field on token response");
            return Err(TokenResponseError::MalformedTokenLine.into());
        }

        let token = token_line
            .trim_start_matches("\"access_token\": \"")
            .trim_end_matches("\",");

        response_lines.next();
        let expires_in_line = response_lines
            .next()
            .ok_or(TokenResponseError::NoExpiresLine)?;

        if !expires_in_line.contains("expires_in") {
            warn!("No expires_in field on token response");
            return Err(TokenResponseError::MalformedExpiresLine.into());
        }

        let expires_in_str = expires_in_line
            .trim_start_matches("\"expires_in\": ")
            .trim_end_matches(",");

        let expires_in = expires_in_str.parse()?;

        Ok((token, expires_in))
    }

    async fn make_token_request(
        https_client: &'b mut ReqwlessClient,
    ) -> Result<HttpRequestHandle<'b, ReqwlessConnection<'b>, &'b [u8]>, ZaptecError> {
        let encoded_body = env!("ZAPTEC_TOKEN_BODY").as_bytes();
        let request = https_client
            .request(Method::POST, formatcp!("{}/oauth/token", ZAPTEC_URL))
            .await?
            .headers(&[("Content-Type", "application/x-www-form-urlencoded")])
            .body(encoded_body)
            .host(ZAPTEC_HOST);
        info!("Token request body: {}", encoded_body);

        Ok(request)
    }

    pub fn token(&'a self) -> &'a ZaptecToken<'a> {
        &self.token
    }
}

impl<'a> ZaptecToken<'a> {
    fn new(token: &'a str, expires_in: u64) -> Self {
        let mut expiration = Instant::now();
        expiration += Duration::from_secs(expires_in);
        Self {
            access_token: token,
            expiration,
        }
    }
}
