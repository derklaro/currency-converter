use axum::body::Body;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{routing, Extension, Router, Server};
use scheduled_thread_pool::ScheduledThreadPool;
use serde::Deserialize;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Deserialize, Debug)]
struct CurrencyApiResponse {
    data: LiraStatus,
}

#[derive(Deserialize, Debug)]
struct LiraStatus {
    #[serde(rename = "EUR")]
    in_eur: f64,
    #[serde(rename = "USD")]
    in_usd: f64,
}

#[derive(Clone, Debug)]
struct LiraInfoHolder(Arc<Mutex<Option<String>>>);

#[tokio::main]
async fn main() -> anyhow::Result<(), anyhow::Error> {
    let lira_info_holder = LiraInfoHolder(Arc::new(Mutex::new(None)));

    // read options
    let bind_host = env::var("BIND").expect("Missing bind host");
    let currency_api_token = env::var("CURRENCY_API_TOKEN").expect("Missing Currency Api Token");

    // schedule lira status updates
    let updating_holder = lira_info_holder.clone();
    let thread_pool = ScheduledThreadPool::new(1);
    thread_pool.execute_at_fixed_rate(
        Duration::from_secs(1),
        Duration::from_secs(10 * 60),
        move || {
            println!("Requesting latest lira information");
            let request_url = format!("https://api.freecurrencyapi.com/v1/latest?apikey={}&currencies=USD%2CEUR&base_currency=TRY", currency_api_token);

            let opt_response = reqwest::blocking::get(request_url);
            if let Ok(response) = opt_response {
                let opt_parsed_body = response.json::<CurrencyApiResponse>();
                if let Ok(response) = opt_parsed_body {
                    let status = format!("Lira status: 1 Lira is equal to {} Euro ({} US-Dollar)", response.data.in_eur, response.data.in_usd);
                    if let Ok(mut guard) = updating_holder.0.lock() {
                        *guard = Some(status);
                    }
                }
            }
        },
    );

    let router: Router<(), Body> = Router::new()
        .route("/status", routing::get(handle_lira_status_request))
        .layer(Extension(lira_info_holder));

    let address = bind_host
        .parse::<SocketAddr>()
        .expect("Unable to parse bind host");
    Server::bind(&address)
        .serve(router.into_make_service())
        .await?;

    Ok(())
}

async fn handle_lira_status_request(
    Extension(holder): Extension<LiraInfoHolder>,
) -> impl IntoResponse {
    match holder.0.lock() {
        Ok(guard) => (
            StatusCode::OK,
            guard.clone().unwrap_or(String::from("No status available")),
        ),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            String::from("Internal error"),
        ),
    }
}
