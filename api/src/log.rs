// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{HISTOGRAM, REQUEST_SOURCE_CLIENT, RESPONSE_STATUS};
use aptos_api_types::X_APTOS_CLIENT;
use aptos_logger::{
    debug, info,
    prelude::{sample, SampleRate},
    warn, Schema,
};
use once_cell::sync::Lazy;
use poem::{http::header, Endpoint, Request, Response, Result};
use poem_openapi::OperationId;
use regex::Regex;
use std::time::Duration;

const REQUEST_SOURCE_CLIENT_UNKNOWN: &str = "unknown";
static REQUEST_SOURCE_CLIENT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"aptos-[a-zA-Z\-]+/[0-9A-Za-z\.\-]+").unwrap());

/// Logs information about the request and response if the response status code
/// is >= 500, to help us debug since this will be an error on our side.
/// We also do general logging of the status code alone regardless of what it is.
pub async fn middleware_log<E: Endpoint>(next: E, request: Request) -> Result<Response> {
    let start = std::time::Instant::now();

    let mut log = HttpRequestLog {
        remote_addr: request.remote_addr().as_socket_addr().cloned(),
        method: request.method().to_string(),
        path: request.uri().path().to_string(),
        status: 0,
        referer: request
            .headers()
            .get(header::REFERER)
            .and_then(|v| v.to_str().ok().map(|v| v.to_string())),
        user_agent: request
            .headers()
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok().map(|v| v.to_string())),
        aptos_client: request
            .headers()
            .get(X_APTOS_CLIENT)
            .and_then(|v| v.to_str().ok().map(|v| v.to_string())),
        elapsed: Duration::from_secs(0),
        forwarded: request
            .headers()
            .get(header::FORWARDED)
            .and_then(|v| v.to_str().ok().map(|v| v.to_string())),
    };

    let response = next.get_response(request).await;

    let elapsed = start.elapsed();

    log.status = response.status().as_u16();
    log.elapsed = elapsed;

    if log.status >= 500 {
        sample!(SampleRate::Duration(Duration::from_secs(1)), warn!(log));
    } else if log.status >= 400 {
        sample!(SampleRate::Duration(Duration::from_secs(60)), info!(log));
    } else {
        sample!(SampleRate::Duration(Duration::from_secs(1)), debug!(log));
    }

    // Log response statuses generally.
    RESPONSE_STATUS
        .with_label_values(&[log.status.to_string().as_str()])
        .observe(elapsed.as_secs_f64());

    // Log response status per-endpoint + method.
    HISTOGRAM
        .with_label_values(&[
            log.method.as_str(),
            response
                .data::<OperationId>()
                .map(|operation_id| operation_id.0)
                .unwrap_or("operation_id_not_set"),
            log.status.to_string().as_str(),
        ])
        .observe(elapsed.as_secs_f64());

    if "submit_transaction"
        == response
            .data::<OperationId>()
            .map(|operation_id| operation_id.0)
            .unwrap_or("operation_id_not_set")
    {
        sample!(
            SampleRate::Duration(Duration::from_secs(5)),
            info!("DEVNET_TRACE (ms): total elapsed: {}", elapsed.as_millis()),
        );
    }

    let operation_start = std::time::Instant::now();
    if "operation_id_not_set"
        == response
            .data::<OperationId>()
            .map(|operation_id| operation_id.0)
            .unwrap_or("operation_id_not_set")
    {
        sample!(
            SampleRate::Duration(Duration::from_secs(5)),
            info!("DEVNET_TRACE (ms): operation_id_not_set: {}: {:?}", operation_start.elapsed().as_millis(), response.data::<OperationId>());
        )
    }

    // Push a counter based on the request source, sliced up by endpoint + method.
    REQUEST_SOURCE_CLIENT
        .with_label_values(&[
            determine_request_source_client(&log.aptos_client),
            response
                .data::<OperationId>()
                .map(|operation_id| operation_id.0)
                .unwrap_or("operation_id_not_set"),
            log.status.to_string().as_str(),
        ])
        .inc();

    Ok(response)
}

// Each of our clients includes a header value called X_APTOS_CLIENT that identifies
// that client. This string follows a particular format: <identifier>/<version>,
// where <identifier> always starts with `aptos-`. This function ensure this string
// matches the specified format and returns it if it does. You can see more specifics
// about how we extract info from the string by looking at the regex we match on.
fn determine_request_source_client(aptos_client: &Option<String>) -> &str {
    // If the header is not set we can't determine the request source.
    let aptos_client = match aptos_client {
        Some(aptos_client) => aptos_client,
        None => return REQUEST_SOURCE_CLIENT_UNKNOWN,
    };

    let determine_start = std::time::Instant::now();
    // If there were no matches, we can't determine the request source. If there are
    // multiple matches for some reason, instead of logging nothing, we use whatever
    // value we matched on last.
    let ret = match REQUEST_SOURCE_CLIENT_REGEX.find_iter(aptos_client).last() {
        Some(capture) => capture.as_str(),
        None => REQUEST_SOURCE_CLIENT_UNKNOWN,
    };

    let elapsed = determine_start.elapsed().as_millis();
    sample!(
        SampleRate::Duration(Duration::from_secs(5)),
        info!("DEVNET_TRACE (ms): determine_request_source_client: {}: {}", determine_start.elapsed().as_millis(), aptos_client);
    );
    if elapsed > 100 {
        sample!(
            SampleRate::Duration(Duration::from_secs(5)),
            info!("DEVNET_TRACE (ms) > 100: determine_request_source_client: {}: {}", determine_start.elapsed().as_millis(), aptos_client);
        )
    }

    ret
}

// TODO: Figure out how to have certain fields be borrowed, like in the
// original implementation.
/// HTTP request log, keeping track of the requests
#[derive(Schema)]
pub struct HttpRequestLog {
    #[schema(display)]
    remote_addr: Option<std::net::SocketAddr>,
    method: String,
    path: String,
    pub status: u16,
    referer: Option<String>,
    user_agent: Option<String>,
    aptos_client: Option<String>,
    #[schema(debug)]
    pub elapsed: std::time::Duration,
    forwarded: Option<String>,
}
