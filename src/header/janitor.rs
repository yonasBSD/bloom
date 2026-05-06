// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use hyper::header::{self, HeaderMap, HeaderName};

use super::response_buckets::HeaderResponseBloomResponseBuckets;
use super::response_ignore::HeaderResponseBloomResponseIgnore;
use super::response_ttl::HeaderResponseBloomResponseTTL;

pub struct HeaderJanitor;

impl HeaderJanitor {
    pub fn clean(headers: &mut HeaderMap) {
        // Collect header names that should be removed
        let headers_remove: Vec<HeaderName> = headers
            .keys()
            .filter(|name| Self::is_contextual(name) || Self::is_internal(name))
            .cloned()
            .collect();

        // Proceed removal (on original headers object)
        for header_remove in headers_remove {
            headers.remove(&header_remove);
        }
    }

    pub fn is_contextual(name: &HeaderName) -> bool {
        name == header::CONNECTION
            || name == header::DATE
            || name == header::UPGRADE
            || name == header::COOKIE
    }

    pub fn is_internal(name: &HeaderName) -> bool {
        name.as_str() == HeaderResponseBloomResponseBuckets::header_name().as_str()
            || name.as_str() == HeaderResponseBloomResponseIgnore::header_name().as_str()
            || name.as_str() == HeaderResponseBloomResponseTTL::header_name().as_str()
    }
}
