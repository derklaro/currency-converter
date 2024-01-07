use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use anyhow::anyhow;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::currency_api::{CurrencyApiClient, CurrencyInfo};

const MAX_CACHE_TIME_SECS: u64 = 5 * 60;
const ONE_USD_RATE: Option<f64> = Some(1.0f64);

#[derive(Deserialize, Debug)]
struct SupportedCurrencies {
    #[serde(alias = "currencies")]
    currency_names: HashMap<String, String>,
}

#[derive(Serialize, Clone, Debug)]
pub(crate) struct CurrencyConvertResult {
    pub base_currency: String,
    pub target_currency: String,
    pub conversion_rate: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct CurrencyConverter {
    api_client: CurrencyApiClient,
    currency_names: HashMap<String, String>,
    fetched_currencies: Arc<RwLock<Option<CurrencyInfo>>>,
}

impl CurrencyConverter {
    pub(crate) fn new(api_client: CurrencyApiClient) -> anyhow::Result<Self> {
        // load currency info
        let currency_info = fs::read_to_string("supported_currencies.json")?;
        let supported = serde_json::from_str::<SupportedCurrencies>(&currency_info)?;

        Ok(Self {
            api_client,
            currency_names: supported.currency_names,
            fetched_currencies: Arc::new(RwLock::new(None)),
        })
    }

    #[cfg(debug_assertions)]
    pub(crate) async fn print_unknown_currency_codes(&self) {
        let current_info = self.get_base_currency_info().await.unwrap();
        let unknown_currencies = current_info
            .currency_rates
            .keys()
            .filter(|code| !self.currency_names.contains_key(*code))
            .cloned()
            .join(", ");
        println!("Unknown currencies: {}", unknown_currencies);
    }

    pub(crate) async fn convert_currencies(
        &self,
        base_currency: String,
        target_currencies: Vec<String>,
    ) -> anyhow::Result<Vec<CurrencyConvertResult>> {
        let mut result = Vec::<CurrencyConvertResult>::with_capacity(target_currencies.len());
        for target_currency in target_currencies {
            let converted_currency = self
                .convert_currency(base_currency.clone(), target_currency)
                .await?;
            result.push(converted_currency);
        }

        Ok(result)
    }

    pub(crate) async fn convert_currency(
        &self,
        base_currency: String,
        target_currency: String,
    ) -> anyhow::Result<CurrencyConvertResult> {
        let current_info = self.get_base_currency_info().await?;

        // get the base and target currency info, if known
        let source_current_rate = self.get_currency_rate(&current_info, &base_currency);
        let target_current_rate = self.get_currency_rate(&current_info, &target_currency);
        if source_current_rate.is_none() || target_current_rate.is_none() {
            return Err(anyhow!("Invalid target or source currency"));
        }

        // convert rates:
        //   1. from usd to source currency
        //   2. from source rate to target
        let source_rate = 1f64 / source_current_rate.unwrap();
        let conversion_rate = source_rate * target_current_rate.unwrap();

        Ok(CurrencyConvertResult {
            base_currency,
            target_currency,
            conversion_rate,
        })
    }

    pub(crate) fn get_currency_name(&self, currency_code: &String) -> String {
        self.currency_names
            .get(currency_code)
            .unwrap_or(currency_code)
            .clone()
    }

    async fn get_base_currency_info(&self) -> anyhow::Result<CurrencyInfo> {
        // double checked locking: check if currency info is present first
        let guard = self.fetched_currencies.read().await;
        if let Some(info) = &*guard {
            if info.timestamp.elapsed().as_secs() <= MAX_CACHE_TIME_SECS {
                return Ok(info.clone());
            }
        }

        // info is not present, drop our current lock and acquire a write
        // lock to fetch and set the value as needed
        drop(guard);

        // re-check if the value is now present to prevent race conditions
        let mut guard = self.fetched_currencies.write().await;
        if let Some(info) = &*guard {
            if info.timestamp.elapsed().as_secs() <= MAX_CACHE_TIME_SECS {
                return Ok(info.clone());
            }
        }

        // info is still not present, fetch it
        let currency_info = self.api_client.fetch_currencies().await?;
        *guard = Some(currency_info.clone());
        Ok(currency_info)
    }

    fn get_currency_rate(
        &self,
        currency_info: &CurrencyInfo,
        currency_code: &String,
    ) -> Option<f64> {
        match currency_code.as_str() {
            "USD" => ONE_USD_RATE,
            _ => currency_info.currency_rates.get(currency_code).cloned(),
        }
    }
}
