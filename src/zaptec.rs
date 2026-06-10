use core::cmp::min;

use crate::{
    error::zaptec::{TokenResponseError, ZaptecError},
    mk_static,
    wifi::{ReqwlessClient, ReqwlessConnection},
};
use const_format::formatcp;
use defmt::info;
use embassy_time::{Duration, Instant};
use reqwless::{
    client::HttpRequestHandle,
    request::{Method, RequestBuilder},
};

const ZAPTEC_URL: &str = "https://api.zaptec.com";
const ZAPTEC_HOST: &str = "api.zaptec.com";

#[derive(defmt::Format)]
pub struct ZaptecSettings {
    pub available_current: f64,
}

pub struct ZaptecClient {
    https_client: ReqwlessClient,
    pub token: ZaptecToken,
    pub installation_id: &'static str,
}

#[derive(defmt::Format)]
pub struct ZaptecToken {
    access_token: &'static str,
    expiration: Instant,
}

impl ZaptecClient {
    pub async fn new(mut https_client: ReqwlessClient) -> Result<Self, ZaptecError> {
        let (token, expires_in) = Self::request_token(&mut https_client).await?;
        let token = ZaptecToken::new(token, expires_in);
        let installation_id = Self::get_installations(&mut https_client, &token).await?;

        Ok(Self {
            https_client,
            token,
            installation_id,
        })
    }

    pub async fn apply_settings(&self, _settings: ZaptecSettings) -> Result<(), ZaptecError> {
        Ok(())
    }

    async fn get_installations<'a>(
        https_client: &'a mut ReqwlessClient,
        token: &'a ZaptecToken,
    ) -> Result<&'static str, ZaptecError> {
        let rx_buf = [0u8; 4096];
        let rx_buf_ref = mk_static!([u8; 4096], rx_buf);
        let mut header_buf = [0u8; 4096];

        let aut_header = token.auth_header(&mut header_buf);
        let headers = &[
            ("accept", "application/json"),
            ("authorization", aut_header),
        ];

        info!("Headers: {}", headers);
        let mut request = https_client
            .request(Method::GET, formatcp!("{}/api/installation", ZAPTEC_URL))
            .await?
            .host(ZAPTEC_HOST)
            .headers(headers);

        let response = request.send(rx_buf_ref).await?;
        let response_body = response.body().read_to_end().await?;
        let response_body_ref = mk_static!(&[u8], response_body);

        str::from_utf8(response_body_ref)?
            .trim_start_matches("{\"Pages\":1,\"Data\":[{\"Id\":\"")
            .split("\"")
            .next()
            .ok_or(ZaptecError::MalformedResponse)
    }

    async fn request_token(
        https_client: &mut ReqwlessClient,
    ) -> Result<(&'static str, u64), ZaptecError> {
        let mut request = Self::make_token_request(https_client).await?;
        let rx_buf = [0u8; 4096];
        let rx_buf_ref = mk_static!([u8; 4096], rx_buf);
        let response = request.send(rx_buf_ref).await?;
        let response_body = response.body().read_to_end().await?;
        let response_body_ref = mk_static!(&[u8], response_body);

        let (token, expires_in) = Self::parse_token_response(str::from_utf8(response_body_ref)?)?;

        Ok((token, expires_in))
    }

    fn parse_token_response(response_body: &str) -> Result<(&str, u64), ZaptecError> {
        let mut response_lines = response_body
            .trim_start_matches("{")
            .trim_end_matches("}")
            .split(",");

        let token = response_lines
            .find_map(|line| {
                if !line.contains("access_token") {
                    return None;
                }
                Some(
                    line.trim()
                        .trim_start_matches("\"access_token\":\"")
                        .trim_end_matches("\"")
                        .trim(),
                )
            })
            .ok_or(TokenResponseError::NoTokenLine)?;

        let expires_in_str = response_lines
            .find_map(|line| {
                if !line.contains("expires_in") {
                    return None;
                }
                Some(line.trim().trim_start_matches("\"expires_in\":").trim())
            })
            .ok_or(TokenResponseError::NoExpiresLine)?;

        let expires_in = expires_in_str.parse()?;

        Ok((token, expires_in))
    }

    const ZAPTEC_TOKEN_BODY: &'static str = formatcp!(
        "grant_type=password&username={}&password={}",
        env!("ZAPTEC_USERNAME"),
        env!("ZAPTEC_PASSWORD")
    );

    async fn make_token_request<'a>(
        https_client: &'a mut ReqwlessClient,
    ) -> Result<HttpRequestHandle<'a, ReqwlessConnection<'a>, &'a [u8]>, ZaptecError> {
        let request = https_client
            .request(Method::POST, formatcp!("{}/oauth/token", ZAPTEC_URL))
            .await?
            .headers(&[("Content-Type", "application/x-www-form-urlencoded")])
            .body(Self::ZAPTEC_TOKEN_BODY.as_bytes())
            .host(ZAPTEC_HOST);

        Ok(request)
    }

    pub fn token(&self) -> &ZaptecToken {
        &self.token
    }
}

impl ZaptecToken {
    pub fn new(token: &'static str, expires_in: u64) -> Self {
        let mut expiration = Instant::now();
        expiration += Duration::from_secs(expires_in);
        Self {
            access_token: token,
            expiration,
        }
    }

    const AUTH_HEADER_PREFIX: &str = "Bearer ";
    pub fn auth_header<'a>(&'a self, buf: &'a mut [u8]) -> &'a str {
        let str = [Self::AUTH_HEADER_PREFIX, self.access_token].concat();
        let dest_len = min(buf.len(), str.len());
        let str_slice = &str.as_bytes()[..dest_len];
        buf[..dest_len].copy_from_slice(str_slice);
        str::from_utf8(buf).unwrap()
    }
}
