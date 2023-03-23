# Lira Status Checker

![License](https://img.shields.io/github/license/derklaro/lira-checker)

HTTP api application that just responds with the current lira status, updated every 15 minutes. A version of this web
application is available at [https://lira.derklaro.dev](https://lira.derklaro.dev)

### Environment variables

1. `BIND`: the address to bind the web server to in form `ip:port` or `[ipv6]:port`.
2. `CURRENCY_API_TOKEN`: an api token for [Free Currency API](https://freecurrencyapi.com). The api will be used to fetch the lira status.

### HTTP routes

#### `/status`:

[Try it out](https://lira.derklaro.dev/status)

Responds with the current lira status, in a nice one-line text format. The response looks like: `Lira status: 1 Lira is equal to 0.048295 Euro (0.052496 US-Dollar)`.

Possible response codes are:

| Status | Meaning                                                                                        |
|--------|------------------------------------------------------------------------------------------------|
| 200    | Ok, but no status has to be available. In that case the response body is `No status available` |
| 500    | An error occurred while processing the request.                                                |

### Compile from source

1. Clone this repository
2. If you're on Linux you might need to install `build-essentials`
3. Make sure you have [Cargo installed](https://doc.rust-lang.org/cargo/getting-started/installation.html) and run `cargo build --release`
4. Take the final file from `target/release/lira-checker[.extension]`
