// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use hyper::header::{HeaderName, HeaderValue};
use std::fmt;

#[derive(Clone)]
pub struct HeaderResponseBloomResponseIgnore();

impl HeaderResponseBloomResponseIgnore {
    pub fn header_name() -> HeaderName {
        HeaderName::from_static("bloom-response-ignore")
    }

    pub fn from_header_value(value: &HeaderValue) -> Option<Self> {
        if value.as_bytes() == b"1" {
            Some(HeaderResponseBloomResponseIgnore())
        } else {
            None
        }
    }
}

impl fmt::Display for HeaderResponseBloomResponseIgnore {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&1, f)
    }
}
