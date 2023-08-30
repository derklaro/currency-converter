mod currency_api;

use crate::currency_api::{CurrencyApiClient, CurrencyApiResponse};
use axum::body::Body;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{routing, Extension, Json, Router, Server};
use moka::future::{Cache, CacheBuilder};
use scheduled_thread_pool::ScheduledThreadPool;
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

    // schedule lira status updates
    schedule_lira_updates(currency_api_client.clone(), currency_info_cache.clone());

    let router: Router<(), Body> = Router::new()
        .route("/status", routing::get(handle_lira_status_request_plain))
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

fn schedule_lira_updates(
    currency_api_client: CurrencyApiClient,
    currency_info_cache: Cache<String, CurrencyApiResponse>,
) {
    let thread_pool = ScheduledThreadPool::new(1);
    thread_pool.execute_at_fixed_rate(Duration::from_secs(1), Duration::from_secs(30), move || {
        println!("Requesting latest lira information");
        if let Ok(response) = currency_api_client.fetch_currency_info_blocking("TRY") {
            currency_info_cache
                .blocking()
                .insert(response.base.to_string(), response);
        }
    });
}

async fn handle_lira_status_request_plain(
    Extension(cache): Extension<Cache<String, CurrencyApiResponse>>,
) -> impl IntoResponse {
    let lira_info_option = cache.get("TRY");
    match lira_info_option {
        None => (StatusCode::NO_CONTENT, String::from("No status available")),
        Some(lira_info) => {
            let in_eur = lira_info.result.rates.get("EUR").unwrap_or(&-1.0);
            let in_usd = lira_info.result.rates.get("USD").unwrap_or(&-1.0);
            let formatted = format!(
                "Lira Status as of {} (UTC): 1 Lira is equal to {} Euro ({} US-Dollar)",
                lira_info.updated, in_eur, in_usd
            );
            (StatusCode::OK, formatted)
        }
    }
}

async fn handle_currency_convert_request(
    Path(base_currency): Path<String>,
    Extension(cache): Extension<Cache<String, CurrencyApiResponse>>,
    Extension(api_client): Extension<CurrencyApiClient>,
    Extension(supported_currencies): Extension<SupportedCurrencies>,
) -> Response {
    // check if the requested currency is supported
    let actual_base_currency = base_currency.to_uppercase();
    if !supported_currencies
        .currencies
        .contains_key(actual_base_currency.as_str())
    {
        return (
            StatusCode::BAD_REQUEST,
            String::from("Base currency is unknown"),
        )
            .into_response();
    }

    // check if the currency information is already cached
    let cached_info = cache.get(actual_base_currency.as_str());
    if let Some(response) = cached_info {
        return (StatusCode::OK, Json(response)).into_response();
    }

    // info is not cached, request from api
    if let Ok(api_response) = api_client
        .fetch_currency_info(actual_base_currency.as_str())
        .await
    {
        cache
            .insert(actual_base_currency, api_response.clone())
            .await;
        return (StatusCode::OK, Json(api_response)).into_response();
    }

    // unable to request information?
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        String::from("Unable to fetch requested info"),
    )
        .into_response()
}
