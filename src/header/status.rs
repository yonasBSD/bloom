// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use std::fmt;

use hyper::header::{HeaderName, HeaderValue};

#[derive(Clone)]
pub enum HeaderBloomStatusValue {
    Hit,
    Miss,
    Direct,
    Reject,
    Offline,
}

#[derive(Clone)]
pub struct HeaderBloomStatus(pub HeaderBloomStatusValue);

impl HeaderBloomStatusValue {
    fn to_str(&self) -> &'static str {
        match *self {
            HeaderBloomStatusValue::Hit => "HIT",
            HeaderBloomStatusValue::Miss => "MISS",
            HeaderBloomStatusValue::Direct => "DIRECT",
            HeaderBloomStatusValue::Reject => "REJECT",
            HeaderBloomStatusValue::Offline => "OFFLINE",
        }
    }
}

impl HeaderBloomStatus {
    pub fn header_name() -> HeaderName {
        HeaderName::from_static("bloom-status")
    }

    pub fn to_header_value(&self) -> HeaderValue {
        HeaderValue::from_static(self.0.to_str())
    }
}

impl fmt::Display for HeaderBloomStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.0.to_str(), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_matches_status_string() {
        assert_eq!(HeaderBloomStatusValue::Hit.to_str(), "HIT");
        assert_eq!(HeaderBloomStatusValue::Miss.to_str(), "MISS");
        assert_eq!(HeaderBloomStatusValue::Direct.to_str(), "DIRECT");
        assert_eq!(HeaderBloomStatusValue::Reject.to_str(), "REJECT");
        assert_eq!(HeaderBloomStatusValue::Offline.to_str(), "OFFLINE");
    }
}
