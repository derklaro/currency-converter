# Lira Status Checker

![License](https://img.shields.io/github/license/derklaro/lira-checker)

HTTP api application that just responds with the current lira status, updated every 30 seconds. A version of this web
application is available at [https://lira.derklaro.dev](https://lira.derklaro.dev)

### Environment variables

1. `BIND`: the address to bind the web server to in form `ip:port` or `[ipv6]:port`.
2. `CURRENCY_API_TOKEN`: an api token for [Fast Forex](https://www.fastforex.io/). The api will be used to fetch the lira status.

### HTTP routes

#### `/status`:

[Try it out](https://lira.derklaro.dev/status)

Responds with the current lira status, in a nice one-line text format. The response looks like: `Lira Status as of 2023-03-24 08:14:40 (UTC): 1 Lira is equal to 0.04859 Euro (0.05245 US-Dollar)`.

Possible response codes are:

| Status | Meaning                                         |
|--------|-------------------------------------------------|
| 200    | Data is available and formatted.                |
| 204    | No data is yet available.                       |
| 500    | An error occurred while processing the request. |

#### `/convert/{base currency}`

[Try it out](https://lira.derklaro.dev/convert/TRY)

Responds with the status of the given base currency in a json encoded format. The response looks like:
```json
{
    "base": "TRY",
    "results": {
        "MZN": 3.34824,
        "BHD": 0.01976,
        "CLF": 0.00117,
        "DZD": 7.13641,
        "ZAR": 0.95221,
        "XCD": 0.14154,
        "GBP": 0.0428,
        "THB": 1.78672,
        "SEK": 0.54328,
        "QAR": 0.19132,
        "MMK": 110.11418,
        "XDR": 0.03908,
        "PLN": 0.22755,
        "TJS": 0.5707,
        ...
    },
    "updated": "2023-03-24 08:15:41"
}
```

Possible response codes are:

| Status | Meaning                                         |
|--------|-------------------------------------------------|
| 200    | Data is available and formatted.                |
| 400    | Invalid base currency.                          |
| 500    | An error occurred while processing the request. |

### Compile from source

1. Clone this repository
2. If you're on Linux you might need to install `build-essentials`
3. Make sure you have [Cargo installed](https://doc.rust-lang.org/cargo/getting-started/installation.html) and run `cargo build --release`
4. Take the final file from `target/release/lira-checker[.extension]`
