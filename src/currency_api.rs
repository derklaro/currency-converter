use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct CurrencyApiResponse {
    pub base: String,
    #[serde(rename = "results")]
    pub result: MultiFetchResult,
    pub updated: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct MultiFetchResult {
    #[serde(flatten)]
    pub rates: HashMap<String, f64>,
}

#[derive(Clone, Debug)]
pub(crate) struct CurrencyApiClient {
    api_token: String,
}

impl CurrencyApiClient {
    pub fn new(api_token: String) -> Self {
        CurrencyApiClient { api_token }
    }

    pub async fn fetch_currency_info(
        &self,
        source_currency: &str,
    ) -> anyhow::Result<CurrencyApiResponse> {
        let request_url = format!(
            "https://api.fastforex.io/fetch-all?from={}&api_key={}",
            source_currency.to_uppercase(),
            self.api_token
        );

        match reqwest::get(request_url).await {
            Ok(response) => response
                .json::<CurrencyApiResponse>()
                .await
                .map_err(Into::into),
            Err(_) => Err(anyhow!("Unable to fetch currency info from api")),
        }
    }
}
