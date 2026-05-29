use crate::wifi::{ReqwlessClient, ReqwlessConnection};
use reqwless::{
    client::HttpRequestHandle,
    headers::ContentType,
    request::{Method, RequestBuilder},
};

const ZAPTEC_URL: &str = "https://api.zaptec.com";
const ZAPTEC_HOST: &str = "api.zaptec.com";
const ZAPTEC_USERNAME: &str = env!("ZAPTEC_USERNAME");
const ZAPTEC_PASSWORD: &str = env!("ZAPTEC_PASSWORD");

pub struct ZaptecSettings {
    pub charging_power: f64,
}

pub struct ZaptecRequester {
    _https_client: ReqwlessClient,
    _token: [u8; 4096],
}

pub enum ZaptecError {}

impl ZaptecRequester {
    pub async fn new(mut https_client: ReqwlessClient) -> Option<Self> {
        let token = Self::request_token(&mut https_client).await?;
        Some(Self {
            _https_client: https_client,
            _token: token,
        })
    }

    pub fn apply_settings(&self, _settings: ZaptecSettings) -> Result<(), ZaptecError> {
        Ok(())
    }

    async fn request_token<'b>(https_client: &'b mut ReqwlessClient) -> Option<[u8; 4096]> {
        let mut request = Self::make_token_request(https_client).await?;
        let mut rx_buf = [0u8; 4096];
        let response = request.send(&mut rx_buf).await.ok()?;
        let response_body = response.body().read_to_end().await.ok()?;
        let response_str = str::from_utf8(response_body).ok()?;

        let token = Self::parse_token_response(response_str)?;

        let mut token_clone = [0u8; 4096];

        token_clone.copy_from_slice(token.as_bytes());

        Some(token_clone)
    }

    fn parse_token_response(response_body: &str) -> Option<&str> {
        let mut response_lines = response_body.lines();
        response_lines.next();
        let token_line = response_lines.next()?;

        if !token_line.contains("access_token") {
            return None;
        }

        let token = token_line
            .trim_start_matches("\"access_token\": \"")
            .trim_end_matches("\",");
        Some(token)
    }

    async fn make_token_request<'b>(
        https_client: &'b mut ReqwlessClient,
    ) -> Option<HttpRequestHandle<'b, ReqwlessConnection<'b>, &'b [u8]>> {
        let body: &[u8] = format_args!(
            "grant_type=password\nusername={}\npassword={}",
            ZAPTEC_USERNAME, ZAPTEC_PASSWORD
        )
        .as_str()?
        .as_bytes();

        let request = https_client
            .request(
                Method::POST,
                format_args!("{}/oauth/token", ZAPTEC_URL).as_str()?,
            )
            .await
            .ok()?
            .body(body)
            .host(ZAPTEC_HOST)
            .content_type(ContentType::ApplicationJson);

        Some(request)
    }
}
