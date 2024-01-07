mod currency_api;
mod currency_convert;

use crate::currency_api::CurrencyApiClient;
use crate::currency_convert::{CurrencyConvertResult, CurrencyConverter};
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{routing, Extension, Router};
use itertools::Itertools;
use std::env;
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // construct currency api client
    let bind_host = env::var("BIND").expect("Missing bind host");
    let ff_api_token = env::var("FF_API_TOKEN").expect("Missing FF Api Token");
    let xe_api_token = env::var("XE_API_TOKEN").expect("Missing XE Api Token");
    let currency_api_client = CurrencyApiClient::new(ff_api_token, xe_api_token);

    // build currency converter
    let currency_converter = CurrencyConverter::new(currency_api_client)?;

    // list currencies that are not yet named (only when running in debug mode)
    #[cfg(debug_assertions)]
    currency_converter.print_unknown_currency_codes().await;

    let router = Router::new()
        .route(
            "/status/:base_currency",
            routing::get(handle_currency_status_request),
        )
        .route(
            "/status/:base_currency/:target_currencies",
            routing::get(handle_currency_status_convert_request),
        )
        .layer(Extension(currency_converter));

    let address = bind_host
        .parse::<SocketAddr>()
        .expect("Unable to parse bind host");
    let listener = TcpListener::bind(address).await?;
    axum::serve(listener, router).await?;

    Ok(())
}

async fn handle_currency_status_request(
    Path(base_currency): Path<String>,
    Extension(converter): Extension<CurrencyConverter>,
) -> impl IntoResponse {
    let upper_base = base_currency.to_uppercase();
    let target_currencies = vec!["EUR".to_string(), "USD".to_string()];
    let converted = converter
        .convert_currencies(upper_base.clone(), target_currencies)
        .await;
    format_currency_response(converter, upper_base, converted)
}

async fn handle_currency_status_convert_request(
    Path((base_currency, target_currencies)): Path<(String, String)>,
    Extension(converter): Extension<CurrencyConverter>,
) -> impl IntoResponse {
    let upper_base = base_currency.to_uppercase();
    let target_currencies: Vec<String> = target_currencies
        .split(',')
        .map(|str| str.trim())
        .map(|str| str.to_uppercase())
        .unique()
        .take(3)
        .collect();
    let converted = converter
        .convert_currencies(upper_base.clone(), target_currencies)
        .await;
    format_currency_response(converter, upper_base, converted)
}

fn format_currency_response(
    converter: CurrencyConverter,
    base_currency: String,
    convert_results: anyhow::Result<Vec<CurrencyConvertResult>>,
) -> impl IntoResponse {
    match convert_results {
        Err(err) => {
            eprintln!("Unable get currency info: {}", err);
            (
                StatusCode::OK,
                String::from("Unable to provide info about requested currencies"),
            )
        }
        Ok(results) => {
            // 1 Turkish Lira is equal to 0.03065 Euro, 0.03353 United States Dollar
            let base_currency_name = converter.get_currency_name(&base_currency);
            let formatted_results = results
                .iter()
                .map(|result| {
                    let currency_name = converter.get_currency_name(&result.target_currency);
                    format!("{:.15} {}", &result.conversion_rate, currency_name)
                })
                .join(", ");

            let formatted_result =
                format!("1 {} is equal to {}", base_currency_name, formatted_results);
            (StatusCode::OK, formatted_result)
        }
    }
}
