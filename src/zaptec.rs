use embedded_io_async::Read;
use reqwless::{
    headers::ContentType,
    request::{Method, RequestBuilder},
};

use crate::wifi::ReqwlessClient;

const ZAPTEC_URL: &str = "https://api.zaptec.com";
const ZAPTEC_HOST: &str = "api.zaptec.com";
const ZAPTEC_USERNAME: &str = env!("ZAPTEC_USERNAME");
const ZAPTEC_PASSWORD: &str = env!("ZAPTEC_PASSWORD");

pub struct ZaptecSettings {
    pub charging_power: f64,
}

pub struct ZaptecRequester {
    https_client: ReqwlessClient,
    token: &'static str,
}

pub enum ZaptecError {}

impl ZaptecRequester {
    pub async fn new(mut https_client: ReqwlessClient) -> Option<Self> {
        let token = Self::request_token(&mut https_client).await?;
        Some(Self {
            https_client,
            token,
        })
    }

    pub fn apply_settings(&self, settings: ZaptecSettings) -> Result<(), ZaptecError> {
        Ok(())
    }

    async fn request_token(https_client: &mut ReqwlessClient) -> Option<&'static str> {
        let body: &[u8] = format_args!(
            "grant_type=password\nusername={}\npassword={}",
            ZAPTEC_USERNAME, ZAPTEC_PASSWORD
        )
        .as_str()?
        .as_bytes();

        let mut rx_buf = [0; 4096];
        let response = https_client
            .request(
                Method::POST,
                format_args!("{}/oauth/token", ZAPTEC_URL).as_str()?,
            )
            .await
            .ok()?
            .body(body)
            .host(ZAPTEC_HOST)
            .content_type(ContentType::ApplicationJson)
            .send(&mut rx_buf)
            .await
            .ok()?;

        let response_body = response.body().read_to_end().await.ok()?;
        let response_str = str::from_utf8(response_body).ok()?;
        let mut response_lines = response_str.lines();
        response_lines.next();
        let token_line = response_lines.next()?;
        Some(
            token_line
                .replace("\"access_token\": \"", "")
                .replace("\",", "")
                .as_ref(),
        )
    }
}
