// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use hyper::header::{HeaderName, HeaderValue};
use std::fmt;

#[derive(Clone)]
pub struct HeaderRequestBloomRequestShard(pub u8);

impl HeaderRequestBloomRequestShard {
    pub fn header_name() -> HeaderName {
        HeaderName::from_static("bloom-request-shard")
    }

    pub fn from_header_value(value: &HeaderValue) -> Option<Self> {
        value
            .to_str()
            .ok()
            .and_then(|value| value.parse::<u8>().ok())
            .map(HeaderRequestBloomRequestShard)
    }
}

impl fmt::Display for HeaderRequestBloomRequestShard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
