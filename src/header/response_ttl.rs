// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use std::fmt;

use hyper::header::{HeaderName, HeaderValue};

#[derive(Clone)]
pub struct HeaderResponseBloomResponseTTL(pub usize);

impl HeaderResponseBloomResponseTTL {
    pub fn header_name() -> HeaderName {
        HeaderName::from_static("bloom-response-ttl")
    }

    pub fn from_header_value(value: &HeaderValue) -> Option<Self> {
        value
            .to_str()
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .map(HeaderResponseBloomResponseTTL)
    }
}

impl fmt::Display for HeaderResponseBloomResponseTTL {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
