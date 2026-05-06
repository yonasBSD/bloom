// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use futures::future::{self, Future};
use httparse;
use hyper::header::{self, HeaderMap, HeaderName, HeaderValue};
use hyper::{Body, Method, Request, Response, StatusCode, Uri, Version};
use itertools::{Itertools, Position};

use super::header::ProxyHeader;
use super::tunnel::ProxyTunnel;
use crate::cache::read::CacheRead;
use crate::cache::route::CacheRoute;
use crate::cache::write::CacheWrite;
use crate::header::janitor::HeaderJanitor;
use crate::header::status::{HeaderBloomStatus, HeaderBloomStatusValue};
use crate::LINE_FEED;

pub struct ProxyServe;

pub type ProxyError = Box<dyn std::error::Error + Send + Sync + 'static>;

const CACHED_PARSE_MAX_HEADERS: usize = 100;

type ProxyServeResult = Result<(String, Option<String>), ()>;
type ProxyServeResultFuture = Box<dyn Future<Item = ProxyServeResult, Error = ()> + Send>;

pub type ProxyServeResponseFuture =
    Box<dyn Future<Item = Response<Body>, Error = ProxyError> + Send>;

impl ProxyServe {
    pub fn handle(req: Request<Body>) -> ProxyServeResponseFuture {
        info!("handled request: {} on {}", req.method(), req.uri().path());

        match req.method() {
            &Method::OPTIONS
            | &Method::HEAD
            | &Method::GET
            | &Method::POST
            | &Method::PATCH
            | &Method::PUT
            | &Method::DELETE => Self::accept(req),
            _ => Self::reject(req, StatusCode::METHOD_NOT_ALLOWED),
        }
    }

    fn accept(req: Request<Body>) -> ProxyServeResponseFuture {
        Self::tunnel(req)
    }

    fn reject(req: Request<Body>, status: StatusCode) -> ProxyServeResponseFuture {
        let mut headers = HeaderMap::new();

        headers.insert(
            HeaderBloomStatus::header_name(),
            HeaderBloomStatus(HeaderBloomStatusValue::Reject).to_header_value(),
        );

        Self::respond(req.method(), status, headers, format!("{}", status))
    }

    fn tunnel(req: Request<Body>) -> ProxyServeResponseFuture {
        let (parts, body) = req.into_parts();

        let method = parts.method;
        let uri = parts.uri;
        let version = parts.version;

        let (headers, auth, shard) = ProxyHeader::parse_from_request(parts.headers);

        let auth_hash = CacheRoute::hash(&auth);

        let origin = headers
            .get(header::ORIGIN)
            .and_then(|origin| origin.to_str().ok());

        let (ns, ns_mask) = CacheRoute::gen_key_cache(
            shard,
            &auth_hash,
            version,
            &method,
            uri.path(),
            uri.query(),
            origin,
        );

        info!("tunneling for ns = {}", ns);

        Box::new(
            Self::fetch_cached_data(shard, &ns, &method, &headers)
                .map_err(|_| Self::make_proxy_error("fetch error"))
                .and_then(move |result| match result {
                    Ok(value) => Self::dispatch_cached(
                        shard, ns, ns_mask, auth_hash, method, uri, version, headers, body,
                        value.0, value.1,
                    ),
                    Err(_) => Self::tunnel_over_proxy(
                        shard, ns, ns_mask, auth_hash, method, uri, version, headers, body,
                    ),
                }),
        )
    }

    fn fetch_cached_data(
        shard: u8,
        ns: &str,
        method: &Method,
        headers: &HeaderMap,
    ) -> ProxyServeResultFuture {
        // Clone inner If-None-Match header value (pass it to future)
        let header_if_none_match = headers
            .get(header::IF_NONE_MATCH)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_owned());

        let ns_string = ns.to_string();

        Box::new(
            CacheRead::acquire_meta(shard, ns, method)
                .and_then(move |result| {
                    match result {
                        Ok(fingerprint) => {
                            debug!(
                                "got fingerprint for cached data = {} on ns = {}",
                                &fingerprint, &ns_string
                            );

                            // Check if not modified?
                            let isnt_modified = match &header_if_none_match {
                                Some(if_none_match_value) => ProxyHeader::check_if_none_match(
                                    if_none_match_value,
                                    &fingerprint,
                                ),
                                None => false,
                            };

                            debug!(
                                "got not modified status for cached data = {} on ns = {}",
                                &isnt_modified, &ns_string
                            );

                            Self::fetch_cached_data_body(ns_string, fingerprint, !isnt_modified)
                        }
                        _ => Box::new(future::ok(Err(()))),
                    }
                })
                .or_else(|_| {
                    error!("failed fetching cached data meta");

                    future::ok(Err(()))
                }),
        )
    }

    fn fetch_cached_data_body(
        ns: String,
        fingerprint: String,
        do_acquire_body: bool,
    ) -> ProxyServeResultFuture {
        // Do not acquire body? (not modified)
        let body_fetcher = if do_acquire_body == false {
            Box::new(future::ok(Ok(None))) as Box<dyn Future<Item = _, Error = ()> + Send>
        } else {
            // Will acquire body (modified)
            CacheRead::acquire_body(&ns)
        };

        Box::new(
            body_fetcher
                .and_then(|body_result| {
                    body_result
                        .or_else(|_| Err(()))
                        .map(|body| Ok((fingerprint, body)))
                })
                .or_else(|_| {
                    error!("failed fetching cached data body");

                    future::ok(Err(()))
                }),
        )
    }

    fn tunnel_over_proxy(
        shard: u8,
        ns: String,
        ns_mask: String,
        auth_hash: String,
        method: Method,
        uri: Uri,
        version: Version,
        headers: HeaderMap,
        body: Body,
    ) -> ProxyServeResponseFuture {
        // Clone method value for closures. Sadly, it looks like Rust borrow \
        //   checker doesnt discriminate properly on this check.
        let method_success = method.to_owned();
        let method_failure = method.to_owned();

        Box::new(
            ProxyTunnel::run(&method, &uri, &headers, body, shard)
                .and_then(move |tunnel_res| {
                    CacheWrite::save(
                        ns,
                        ns_mask,
                        auth_hash,
                        shard,
                        method,
                        version,
                        tunnel_res.status(),
                        tunnel_res.headers().to_owned(),
                        tunnel_res.into_body(),
                    )
                })
                .and_then(move |mut result| match result.body {
                    Ok(body_string) => Self::dispatch_fetched(
                        &method_success,
                        &result.status,
                        result.headers,
                        HeaderBloomStatusValue::Miss,
                        body_string,
                        result.fingerprint,
                    ),
                    Err(body_string_values) => {
                        match body_string_values {
                            Some(body_string) => {
                                // Enforce clean headers, as usually they get \
                                //   cleaned from cache writer
                                HeaderJanitor::clean(&mut result.headers);

                                Self::dispatch_fetched(
                                    &method_success,
                                    &result.status,
                                    result.headers,
                                    HeaderBloomStatusValue::Direct,
                                    body_string,
                                    result.fingerprint,
                                )
                            }
                            _ => Self::dispatch_failure(&method_success),
                        }
                    }
                })
                .or_else(move |_| Self::dispatch_failure(&method_failure)),
        )
    }

    fn dispatch_cached(
        shard: u8,
        ns: String,
        ns_mask: String,
        auth_hash: String,
        method: Method,
        req_uri: Uri,
        req_version: Version,
        req_headers: HeaderMap,
        req_body: Body,
        res_fingerprint: String,
        res_string: Option<String>,
    ) -> ProxyServeResponseFuture {
        // Response modified? (non-empty body)
        if let Some(res_string_value) = res_string {
            let mut headers = [httparse::EMPTY_HEADER; CACHED_PARSE_MAX_HEADERS];
            let mut res = httparse::Response::new(&mut headers);

            // Split headers from body
            let body = Self::parse_response_body(&res_string_value);

            match res.parse(res_string_value.as_bytes()) {
                Ok(_) => {
                    // Process cached status
                    let code = res.code.unwrap_or(500u16);
                    let status =
                        StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                    // Process cached headers
                    let mut headers = HeaderMap::new();

                    for header in res.headers {
                        if let (Ok(header_name), Ok(header_value)) = (
                            HeaderName::from_bytes(header.name.as_bytes()),
                            HeaderValue::from_bytes(header.value),
                        ) {
                            headers.insert(header_name, header_value);
                        }
                    }

                    ProxyHeader::set_etag(&mut headers, &res_fingerprint);

                    headers.insert(
                        HeaderBloomStatus::header_name(),
                        HeaderBloomStatus(HeaderBloomStatusValue::Hit).to_header_value(),
                    );

                    // Serve cached response
                    Self::respond(&method, status, headers, body)
                }
                Err(err) => {
                    error!("failed parsing cached response: {}", err);

                    Self::tunnel_over_proxy(
                        shard,
                        ns,
                        ns_mask,
                        auth_hash,
                        method,
                        req_uri,
                        req_version,
                        req_headers,
                        req_body,
                    )
                }
            }
        } else {
            // Response not modified for client, process non-modified + cached headers
            let mut headers = HeaderMap::new();

            ProxyHeader::set_etag(&mut headers, &res_fingerprint);

            headers.insert(
                HeaderBloomStatus::header_name(),
                HeaderBloomStatus(HeaderBloomStatusValue::Hit).to_header_value(),
            );

            // Serve non-modified response
            Self::respond(&method, StatusCode::NOT_MODIFIED, headers, String::from(""))
        }
    }

    fn dispatch_fetched(
        method: &Method,
        status: &StatusCode,
        mut headers: HeaderMap,
        bloom_status: HeaderBloomStatusValue,
        body_string: String,
        fingerprint: Option<String>,
    ) -> ProxyServeResponseFuture {
        // Process ETag for content?
        if let Some(fingerprint_value) = fingerprint {
            ProxyHeader::set_etag(&mut headers, &fingerprint_value);
        }

        headers.insert(
            HeaderBloomStatus::header_name(),
            HeaderBloomStatus(bloom_status).to_header_value(),
        );

        Self::respond(method, *status, headers, body_string)
    }

    fn dispatch_failure(method: &Method) -> ProxyServeResponseFuture {
        let status = StatusCode::BAD_GATEWAY;

        let mut headers = HeaderMap::new();

        headers.insert(
            HeaderBloomStatus::header_name(),
            HeaderBloomStatus(HeaderBloomStatusValue::Offline).to_header_value(),
        );

        Self::respond(method, status, headers, format!("{}", status))
    }

    fn parse_response_body(res_string_value: &str) -> String {
        let (mut body, mut is_last_line_empty) = (String::new(), false);

        // Scan response lines
        let lines = res_string_value.lines().with_position();

        for (position, line) in lines {
            if body.is_empty() == false || is_last_line_empty == true {
                // Append line to body
                body.push_str(line);

                // Append line feed character?
                if let Position::First | Position::Middle = position {
                    body.push_str(LINE_FEED);
                }
            }

            is_last_line_empty = line.is_empty();
        }

        body
    }

    fn make_proxy_error(msg: &'static str) -> ProxyError {
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg))
    }

    fn respond(
        method: &Method,
        status: StatusCode,
        headers: HeaderMap,
        body_string: String,
    ) -> ProxyServeResponseFuture {
        Box::new(future::ok({
            let body = match method {
                &Method::GET | &Method::POST | &Method::PATCH | &Method::PUT | &Method::DELETE => {
                    Body::from(body_string)
                }
                _ => Body::empty(),
            };

            let mut response = Response::new(body);

            *response.status_mut() = status;
            *response.headers_mut() = headers;

            response
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_parses_response_body() {
        let body = "2022-10-03";
        let headers =
            "Content-Type: text/plain; charset=utf-8\nServer: Kestrel\nTransfer-Encoding: chunked";

        let response_string = format!("{headers}\n\n{body}");

        assert_eq!(body, ProxyServe::parse_response_body(&response_string));
    }
}
