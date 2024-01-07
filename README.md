# Currency Converter Service

![License](https://img.shields.io/github/license/derklaro/currency-converter)

A very small application that allows to convert information between multiple currencies. A version of this web
application is available at [https://currency.derklaro.dev](https://currency.derklaro.dev).

### Environment variables

1. `BIND`: the address to bind the web server to in form `ip:port` or `[ipv6]:port`.
2. `CURRENCY_API_TOKEN`: an api token for [Fast Forex](https://www.fastforex.io/). The api will be used to fetch the
   currency states.

### HTTP routes

#### `/status/{base currency}`:

[Try it out](https://currency.derklaro.dev/status/TRY)

Responds with the status of the given currency based on Euro and US-Dollar. The response looks
like: `Status as of 2023-08-30 14:00:16 (UTC): 1 Turkish Lira is equal to 0.03422 Euro, 0.03744 United States Dollar`.

Possible response codes are:

| Status | Meaning                                           |
|--------|---------------------------------------------------|
| 200    | Data is available and formatted.                  |
| 204    | No data is available for the given base currency. |
| 5XX    | An error occurred while processing the request.   |

#### `/status/{base currency}/{target currencies}`:

[Try it out](https://currency.derklaro.dev/status/TRY/SOS,EUR)

Responds with the status of the given currency based on the given other currencies (comma-separated, up to 3 are
supported). The response looks
like: `Status as of 2023-08-30 14:00:16 (UTC): 1 Turkish Lira is equal to 21.28689 Somali Shilling, 0.03422 Euro`.

Possible response codes are:

| Status | Meaning                                           |
|--------|---------------------------------------------------|
| 200    | Data is available and formatted.                  |
| 204    | No data is available for the given base currency. |
| 5XX    | An error occurred while processing the request.   |

### Compile from source

1. Clone this repository
2. If you're on Linux you might need to install `build-essentials`
3. Make sure you have [Cargo installed](https://doc.rust-lang.org/cargo/getting-started/installation.html) and
   run `cargo build --release`
4. Take the final file from `target/release/currency-converter[.extension]`
