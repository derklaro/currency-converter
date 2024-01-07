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
    ff_api_token: String,
    xe_api_token: String,
    api_rest_client: Client,
}

impl CurrencyApiClient {
    pub fn new(ff_api_token: String, xe_api_token: String) -> Self {
        let api_rest_client = Client::builder()
            .https_only(true)
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .expect("Unable to build rest api client");
        CurrencyApiClient {
            ff_api_token,
            xe_api_token,
            api_rest_client,
        }
    }

    pub async fn fetch_currencies(&self) -> anyhow::Result<CurrencyInfo> {
        let fastforex_info = self.fetch_fastforex_info().await?;
        let xe_info = self.fetch_xe_rates().await?;

        let mut result = fastforex_info.currency_rates;
        for (currency, rate) in xe_info.currency_rates.into_iter() {
            result.entry(currency).or_insert(rate);
        }

        Ok(CurrencyInfo {
            timestamp: Instant::now(),
            currency_rates: result,
        })
    }

    async fn fetch_fastforex_info(&self) -> anyhow::Result<CurrencyApiResponse> {
        let request_url = format!(
            "https://api.fastforex.io/fetch-all?from=USD&api_key={}",
            self.ff_api_token
        );

        match self.api_rest_client.get(request_url).send().await {
            Ok(response) => response
                .json::<CurrencyApiResponse>()
                .await
                .map_err(Into::into),
            Err(_) => Err(anyhow!("Unable to fetch currency info from FastForex")),
        }
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
