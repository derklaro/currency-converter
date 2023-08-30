mod currency_api;

use crate::currency_api::{CurrencyApiClient, CurrencyApiResponse};
use axum::body::Body;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{routing, Extension, Json, Router, Server};
use moka::future::{Cache, CacheBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::time::Duration;

#[derive(Deserialize, Serialize, Clone, Debug)]
struct SupportedCurrencies {
    currencies: HashMap<String, String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    let currency_info_cache: Cache<String, CurrencyApiResponse> = CacheBuilder::default()
        .name("currency")
        .initial_capacity(5)
        .time_to_live(Duration::from_secs(5 * 60))
        .build();

    // read options
    let bind_host = env::var("BIND").expect("Missing bind host");
    let currency_api_token = env::var("CURRENCY_API_TOKEN").expect("Missing Currency Api Token");
    let currency_api_client = CurrencyApiClient::new(currency_api_token);

    // parse supported currencies information
    let supported_currencies_file_content = read_file_content("supported_currencies.json")?;
    let supported_currencies: SupportedCurrencies =
        serde_json::from_str(&supported_currencies_file_content)?;

    let router: Router<(), Body> = Router::new()
        .route(
            "/status/:base_currency",
            routing::get(handle_currency_status_request),
        )
        .route(
            "/convert/:base_currency",
            routing::get(handle_currency_convert_request),
        )
        .layer(Extension(currency_info_cache))
        .layer(Extension(currency_api_client))
        .layer(Extension(supported_currencies));

    let address = bind_host
        .parse::<SocketAddr>()
        .expect("Unable to parse bind host");
    Server::bind(&address)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

fn read_file_content<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<String> {
    let mut supported_currencies_file = File::open(path)?;
    let mut file_content = String::new();
    supported_currencies_file.read_to_string(&mut file_content)?;
    Ok(file_content)
}

async fn handle_currency_status_request(
    Path(base_currency): Path<String>,
    Extension(cache): Extension<Cache<String, CurrencyApiResponse>>,
    Extension(api_client): Extension<CurrencyApiClient>,
    Extension(supported_currencies): Extension<SupportedCurrencies>,
) -> impl IntoResponse {
    match get_currency_info(&base_currency, &api_client, &supported_currencies, &cache).await {
        Some(response) => {
            // requested currency name must be available at this point
            let requested_currency = supported_currencies
                .currencies
                .get(base_currency.to_uppercase().as_str())
                .unwrap();

            // extract the information about the currency in euro and usd
            let in_eur = response.result.rates.get("EUR").unwrap_or(&-1.0);
            let in_usd = response.result.rates.get("USD").unwrap_or(&-1.0);

            // build the formatted string to return
            let formatted = format!(
                "Status as of {} (UTC): 1 {} is equal to {} Euro ({} US-Dollar)",
                response.updated, requested_currency, in_eur, in_usd
            );
            (StatusCode::OK, formatted)
        }
        None => (
            StatusCode::NO_CONTENT,
            String::from("No info for currency available"),
        ),
    }
}

async fn handle_currency_convert_request(
    Path(base_currency): Path<String>,
    Extension(cache): Extension<Cache<String, CurrencyApiResponse>>,
    Extension(api_client): Extension<CurrencyApiClient>,
    Extension(supported_currencies): Extension<SupportedCurrencies>,
) -> Response {
    match get_currency_info(&base_currency, &api_client, &supported_currencies, &cache).await {
        Some(response) => (StatusCode::OK, Json(response)).into_response(),
        None => (
            StatusCode::NO_CONTENT,
            String::from("No info for currency available"),
        )
            .into_response(),
    }
}

async fn get_currency_info(
    currency: &String,
    currency_api_client: &CurrencyApiClient,
    supported_currencies: &SupportedCurrencies,
    currency_info_cache: &Cache<String, CurrencyApiResponse>,
) -> Option<CurrencyApiResponse> {
    // check if the requested currency is supported
    let actual_base_currency = currency.to_uppercase();
    if !supported_currencies
        .currencies
        .contains_key(actual_base_currency.as_str())
    {
        return None;
    }

    // check if the currency information is already cached
    let cached_info = currency_info_cache.get(actual_base_currency.as_str());
    if cached_info.is_some() {
        return cached_info;
    }

    // info is not cached, request from api
    if let Ok(api_response) = currency_api_client
        .fetch_currency_info(actual_base_currency.as_str())
        .await
    {
        currency_info_cache
            .insert(actual_base_currency, api_response.clone())
            .await;
        return Some(api_response);
    }

    // unable to request information?
    None
}
