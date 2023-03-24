use axum::body::Body;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{routing, Extension, Json, Router, Server};
use scheduled_thread_pool::ScheduledThreadPool;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Deserialize, Serialize, Clone, Debug)]
struct CurrencyApiResponse {
    base: String,
    #[serde(rename = "results")]
    result: MultiFetchResult,
    updated: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct MultiFetchResult {
    #[serde(flatten)]
    rates: HashMap<String, f64>,
}

#[derive(Clone, Debug)]
struct LiraInfoHolder(Arc<Mutex<Option<CurrencyApiResponse>>>);

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    let lira_info_holder = LiraInfoHolder(Arc::new(Mutex::new(None)));

    // read options
    let bind_host = env::var("BIND").expect("Missing bind host");
    let currency_api_token = env::var("CURRENCY_API_TOKEN").expect("Missing Currency Api Token");

    // schedule lira status updates
    let updating_holder = lira_info_holder.clone();
    let thread_pool = ScheduledThreadPool::new(1);
    thread_pool.execute_at_fixed_rate(Duration::from_secs(1), Duration::from_secs(30), move || {
        println!("Requesting latest lira information");
        let request_url = format!(
            "https://api.fastforex.io/fetch-all?from={}&api_key={}",
            "TRY", currency_api_token
        );

        let opt_response = reqwest::blocking::get(request_url);
        match opt_response {
            Ok(response) => {
                let opt_parsed_body = response.json::<CurrencyApiResponse>();
                if let Ok(response) = opt_parsed_body {
                    if let Ok(mut guard) = updating_holder.0.lock() {
                        *guard = Some(response);
                    }
                }
            }
            Err(err) => println!("Unable to fetch lira data: {}", err),
        }
    });

    let router: Router<(), Body> = Router::new()
        .route("/status", routing::get(handle_lira_status_request_plain))
        .route(
            "/status/json",
            routing::get(handle_lira_status_request_json),
        )
        .layer(Extension(lira_info_holder));

    let address = bind_host
        .parse::<SocketAddr>()
        .expect("Unable to parse bind host");
    Server::bind(&address)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

async fn handle_lira_status_request_plain(
    Extension(holder): Extension<LiraInfoHolder>,
) -> impl IntoResponse {
    match holder.0.lock() {
        Ok(guard) => guard
            .as_ref()
            .map(|data| {
                let in_eur = data.result.rates.get("EUR").unwrap_or(&-1.0);
                let in_usd = data.result.rates.get("USD").unwrap_or(&-1.0);
                let formatted = format!(
                    "Lira Status as of {} (UTC): 1 Lira is equal to {} Euro ({} US-Dollar)",
                    data.updated, in_eur, in_usd
                );
                (StatusCode::OK, formatted)
            })
            .unwrap_or((StatusCode::NO_CONTENT, String::from("No status available"))),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            String::from("Unable to get latest data"),
        ),
    }
}

async fn handle_lira_status_request_json(Extension(holder): Extension<LiraInfoHolder>) -> Response {
    match holder.0.lock() {
        Ok(guard) => guard
            .as_ref()
            .map(|data| (StatusCode::OK, Json(data.clone())).into_response())
            .unwrap_or(
                (StatusCode::NO_CONTENT, String::from("No status available")).into_response(),
            ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            String::from("Unable to get latest data"),
        )
            .into_response(),
    }
}
