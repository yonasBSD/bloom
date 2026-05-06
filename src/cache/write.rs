// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use std::future::Future;
use std::pin::Pin;

use farmhash;
use hyper::{Body, HeaderMap, Method, StatusCode, Version};

use super::check::CacheCheck;
use super::route::CacheRoute;
use crate::header::janitor::HeaderJanitor;
use crate::header::response_buckets::HeaderResponseBloomResponseBuckets;
use crate::header::response_ttl::HeaderResponseBloomResponseTTL;
use crate::proxy::serve::ProxyError;
use crate::APP_CACHE_STORE;
use crate::APP_CONF;

pub struct CacheWrite;

pub struct CacheWriteResult {
    pub body: Result<String, Option<String>>,
    pub fingerprint: Option<String>,
    pub status: StatusCode,
    pub headers: HeaderMap,
}

pub type CacheWriteResultFuture =
    Pin<Box<dyn Future<Output = Result<CacheWriteResult, ProxyError>> + Send>>;

impl CacheWrite {
    pub fn save(
        key: String,
        key_mask: String,
        auth_hash: String,
        shard: u8,
        method: Method,
        version: Version,
        status: StatusCode,
        mut headers: HeaderMap,
        body: Body,
    ) -> CacheWriteResultFuture {
        Box::pin(async move {
            let bytes = hyper::body::to_bytes(body)
                .await
                .map_err(|err| -> ProxyError { Box::new(err) })?;

            let body_result = String::from_utf8(bytes.to_vec());

            if let Ok(body_value) = body_result {
                debug!("checking whether to write cache for key: {}", &key);

                if APP_CONF.cache.disable_write == false
                    && CacheCheck::from_response(&method, &status, &headers) == true
                {
                    debug!("key: {} cacheable, writing cache", &key);

                    // Acquire bucket from response, or fallback to no bucket
                    let mut key_tags =
                        match headers.get(HeaderResponseBloomResponseBuckets::header_name()) {
                            None => Vec::new(),
                            Some(value) => {
                                match HeaderResponseBloomResponseBuckets::from_header_value(value) {
                                    None => Vec::new(),
                                    Some(buckets) => buckets
                                        .0
                                        .iter()
                                        .map(|value| {
                                            CacheRoute::gen_key_bucket_from_hash(
                                                shard,
                                                &CacheRoute::hash(value),
                                            )
                                        })
                                        .collect::<Vec<(String, String)>>(),
                                }
                            }
                        };

                    key_tags.push(CacheRoute::gen_key_auth_from_hash(shard, &auth_hash));

                    // Acquire TTL from response, or fallback to default TTL
                    let ttl = match headers.get(HeaderResponseBloomResponseTTL::header_name()) {
                        None => APP_CONF.cache.ttl_default,
                        Some(value) => {
                            match HeaderResponseBloomResponseTTL::from_header_value(value) {
                                None => APP_CONF.cache.ttl_default,
                                Some(ttl) => ttl.0,
                            }
                        }
                    };

                    // Clean headers before they get stored
                    HeaderJanitor::clean(&mut headers);

                    // Generate storable value
                    let body_string = format!(
                        "{}\n{}\n{}",
                        CacheWrite::generate_chain_banner(&version, &status),
                        CacheWrite::generate_chain_headers(&headers),
                        body_value
                    );

                    // Process value fingerprint
                    let fingerprint = Self::process_body_fingerprint(&body_string);

                    // Write to cache
                    let result = APP_CACHE_STORE
                        .set(key, key_mask, body_string, fingerprint, ttl, key_tags)
                        .await;

                    match result {
                        Ok(fingerprint) => {
                            debug!("wrote cache");

                            Ok(CacheWriteResult {
                                body: Ok(body_value),
                                fingerprint: Some(fingerprint),
                                status,
                                headers,
                            })
                        }
                        Err(forward) => {
                            warn!("could not write cache because: {:?}", forward.0);

                            Ok(CacheWriteResult {
                                body: Err(Some(body_value)),
                                fingerprint: Some(forward.1),
                                status,
                                headers,
                            })
                        }
                    }
                } else {
                    debug!("key: {} not cacheable, ignoring", &key);

                    // Not cacheable, ignore
                    Ok(Self::result_cache_write_error(
                        Some(body_value),
                        status,
                        headers,
                    ))
                }
            } else {
                error!("failed unwrapping body value for key: {}, ignoring", &key);

                // Not cacheable, ignore
                Ok(Self::result_cache_write_error(None, status, headers))
            }
        })
    }

    fn generate_chain_banner(version: &Version, status: &StatusCode) -> String {
        format!("{:?} {}", *version, status)
    }

    fn generate_chain_headers(headers: &HeaderMap) -> String {
        headers
            .iter()
            .filter(|(name, _)| HeaderJanitor::is_contextual(name) == false)
            .map(|(name, value)| format!("{}: {}\n", name, value.to_str().unwrap_or("")))
            .collect()
    }

    fn process_body_fingerprint(body_string: &str) -> String {
        format!("{:x}", farmhash::fingerprint64(body_string.as_bytes()))
    }

    fn result_cache_write_error(
        body: Option<String>,
        status: StatusCode,
        headers: HeaderMap,
    ) -> CacheWriteResult {
        CacheWriteResult {
            body: Err(body),
            fingerprint: None,
            status,
            headers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn it_fails_saving_cache() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            assert!(CacheWrite::save(
                "bloom:0:c:90d52bc6:f773d6f1".to_string(),
                "90d52bc6:f773d6f1".to_string(),
                "90d52bc6".to_string(),
                0,
                Method::GET,
                Version::HTTP_11,
                StatusCode::OK,
                HeaderMap::new(),
                Body::empty(),
            )
            .await
            .is_err());
        });
    }
}
