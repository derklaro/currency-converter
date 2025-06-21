use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::anyhow;
use reqwest::Client;
use serde::Deserialize;

#[derive(Clone, Debug)]
pub(crate) struct CurrencyInfo {
    pub timestamp: Instant,
    pub currency_rates: HashMap<String, f64>,
}

#[derive(Deserialize, Clone, Debug)]
struct CurrencyApiResponse {
    #[serde(alias = "rates", alias = "results")]
    currency_rates: HashMap<String, f64>,
}

#[derive(Clone, Debug)]
pub(crate) struct CurrencyApiClient {
    xe_api_token: String,
    api_rest_client: Client,
}

impl CurrencyApiClient {
    pub fn new(xe_api_token: String) -> Self {
        let api_rest_client = Client::builder()
            .https_only(true)
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .expect("Unable to build rest api client");
        CurrencyApiClient {
            xe_api_token,
            api_rest_client,
        }
    }

    pub async fn fetch_currencies(&self) -> anyhow::Result<CurrencyInfo> {
        let xe_info = self.fetch_xe_rates().await?;
        Ok(CurrencyInfo {
            timestamp: Instant::now(),
            currency_rates: xe_info.currency_rates,
        })
    }

    async fn fetch_xe_rates(&self) -> anyhow::Result<CurrencyApiResponse> {
        let request_result = self
            .api_rest_client
            .get("https://www.xe.com/api/protected/midmarket-converter/")
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Basic {}", &self.xe_api_token),
            )
            .send()
            .await;

        match request_result {
            Ok(response) => response
                .json::<CurrencyApiResponse>()
                .await
                .map_err(Into::into),
            Err(_) => Err(anyhow!("Unable to fetch currency info from XE")),
        }
    }
}
