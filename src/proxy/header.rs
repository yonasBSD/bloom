// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use hyper::header::{self, HeaderMap, HeaderValue};
use std::str::from_utf8;

use super::defaults;
use crate::header::request_shard::HeaderRequestBloomRequestShard;
use crate::APP_CONF;

pub struct ProxyHeader;

impl ProxyHeader {
    pub fn parse_from_request(headers: HeaderMap) -> (HeaderMap, String, u8) {
        // Request header: 'Authorization'
        let auth = match headers.get(header::AUTHORIZATION) {
            None => defaults::REQUEST_AUTHORIZATION_DEFAULT,
            Some(value) => {
                from_utf8(value.as_bytes()).unwrap_or(defaults::REQUEST_AUTHORIZATION_DEFAULT)
            }
        }
        .to_string();

        // Request header: 'Bloom-Request-Shard'
        let shard = match headers.get(HeaderRequestBloomRequestShard::header_name()) {
            None => APP_CONF.proxy.shard_default,
            Some(value) => HeaderRequestBloomRequestShard::from_header_value(value)
                .map(|header| header.0)
                .unwrap_or(APP_CONF.proxy.shard_default),
        };

        (headers, auth, shard)
    }

    pub fn set_etag(headers: &mut HeaderMap, fingerprint: &str) {
        headers.insert(header::VARY, HeaderValue::from_static("ETag"));

        headers.insert(
            header::ETAG,
            HeaderValue::from_str(&format!("\"{}\"", fingerprint)).unwrap(),
        );
    }

    pub fn check_if_none_match(if_none_match: &str, fingerprint: &str) -> bool {
        let value = if_none_match.trim();

        if value == "*" {
            return true;
        }

        for etag in value.split(',') {
            let etag = etag.trim();

            let etag = if etag.starts_with("W/") {
                &etag[2..]
            } else {
                etag
            };

            let etag = etag.trim_matches('"');

            if etag == fingerprint {
                return true;
            }
        }

        false
    }
}
