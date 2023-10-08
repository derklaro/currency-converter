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
            "/status/:base_currency/:target_currencies",
            routing::get(handle_currency_status_convert_request),
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
    convert_currency_info_to(
        &base_currency,
        &String::from("EUR,USD"),
        api_client,
        &supported_currencies,
        &cache,
    )
    .await
}

async fn handle_currency_status_convert_request(
    Path((base_currency, target_currencies)): Path<(String, String)>,
    Extension(cache): Extension<Cache<String, CurrencyApiResponse>>,
    Extension(api_client): Extension<CurrencyApiClient>,
    Extension(supported_currencies): Extension<SupportedCurrencies>,
) -> impl IntoResponse {
    convert_currency_info_to(
        &base_currency,
        &target_currencies,
        api_client,
        &supported_currencies,
        &cache,
    )
    .await
}

async fn convert_currency_info_to(
    currency: &str,
    target_currencies: &str,
    currency_api_client: CurrencyApiClient,
    supported_currencies: &SupportedCurrencies,
    currency_info_cache: &Cache<String, CurrencyApiResponse>,
) -> impl IntoResponse {
    match get_currency_info(
        currency,
        currency_api_client,
        supported_currencies,
        currency_info_cache,
    )
    .await
    {
        Some(response) => {
            // requested currency name must be available at this point
            let requested_currency = supported_currencies
                .currencies
                .get(currency.to_uppercase().as_str())
                .unwrap();

            // extract the information about the target currencies using the requested one as the base info
            let extracted_currency_targets: Vec<(&String, &f64)> = target_currencies
                .split(',')
                .take(3)
                .map(|str| str.trim())
                .map(|currency_target| {
                    (
                        currency_target,
                        response
                            .result
                            .rates
                            .get(currency_target.to_uppercase().as_str()),
                    )
                })
                .filter(|tuple| tuple.1.is_some())
                .map(|tuple| (tuple.0, tuple.1.unwrap()))
                .filter_map(|tuple| {
                    supported_currencies
                        .currencies
                        .get(tuple.0.to_uppercase().as_str())
                        .map(|currency_name| (currency_name, tuple.1))
                })
                .collect();
            if extracted_currency_targets.is_empty() {
                return (
                    StatusCode::OK,
                    String::from("No supported currency given to convert to"),
                );
            }

            // format and return the response for the request
            let formatted_targets = extracted_currency_targets
                .into_iter()
                .map(|target_info| format!("{} {}", target_info.1, target_info.0))
                .collect::<Vec<String>>()
                .join(", ");
            let formatted_base_info = format!(
                "Status as of {} (UTC): 1 {} is equal to {}",
                response.updated, requested_currency, formatted_targets
            );
            (StatusCode::OK, formatted_base_info)
        }
        None => (
            StatusCode::OK,
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
    match get_currency_info(&base_currency, api_client, &supported_currencies, &cache).await {
        Some(response) => (StatusCode::OK, Json(response)).into_response(),
        None => (
            StatusCode::NO_CONTENT,
            String::from("No info for currency available"),
        )
            .into_response(),
    }
}

async fn get_currency_info(
    currency: &str,
    currency_api_client: CurrencyApiClient,
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
    let cached_info = currency_info_cache.get(actual_base_currency.as_str()).await;
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
