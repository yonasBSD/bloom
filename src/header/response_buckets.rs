// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use hyper::header::{HeaderName, HeaderValue};
use std::fmt;

#[derive(Clone)]
pub struct HeaderResponseBloomResponseBuckets(pub Vec<String>);

impl HeaderResponseBloomResponseBuckets {
    pub fn header_name() -> HeaderName {
        HeaderName::from_static("bloom-response-buckets")
    }

    pub fn from_header_value(value: &HeaderValue) -> Option<Self> {
        value
            .to_str()
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .map(|bucket| bucket.trim().to_string())
                    .filter(|bucket| !bucket.is_empty())
                    .collect()
            })
            .map(HeaderResponseBloomResponseBuckets)
    }
}

impl fmt::Display for HeaderResponseBloomResponseBuckets {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0.join(", "))
    }
}
