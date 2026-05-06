// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use futures::{future, Future};
use hyper::client::HttpConnector;
use hyper::header::HeaderMap;
use hyper::{Body, Client, Method, Request, Response, Uri};
use std::time::Duration;

use super::serve::ProxyError;
use crate::APP_CONF;

const MAX_SHARDS: u8 = 16;
const CLIENT_KEEP_ALIVE_TIMEOUT_SECONDS: u64 = 30;

lazy_static! {
    static ref SHARD_REGISTER: [Option<Uri>; MAX_SHARDS as usize] = map_shards();
}

thread_local! {
    static TUNNEL_CLIENT: Client<HttpConnector> = make_client();
}

pub struct ProxyTunnel;

pub type ProxyTunnelFuture = Box<dyn Future<Item = Response<Body>, Error = ProxyError> + Send>;

fn make_client() -> Client<HttpConnector> {
    Client::builder()
        .keep_alive(true)
        .keep_alive_timeout(Duration::from_secs(CLIENT_KEEP_ALIVE_TIMEOUT_SECONDS))
        .build(HttpConnector::new(APP_CONF.proxy.tunnel_clients as usize))
}

fn map_shards() -> [Option<Uri>; MAX_SHARDS as usize] {
    // Notice: this array cannot be initialized using the short format, as hyper::Uri doesnt \
    //   implement the Copy trait, hence the ugly hardcoded initialization vector w/ Nones.
    let mut shards = [
        None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
        None,
    ];

    for shard in &APP_CONF.proxy.shard {
        // Shard number overflows?
        if shard.shard >= MAX_SHARDS {
            panic!("shard number overflows maximum of {} shards", MAX_SHARDS);
        }

        // Store this shard
        shards[shard.shard as usize] = Some(
            format!("http://{}:{}", shard.host, shard.port)
                .parse()
                .expect("could not build shard uri"),
        );
    }

    shards
}

impl ProxyTunnel {
    pub fn run(
        method: &Method,
        uri: &Uri,
        headers: &HeaderMap,
        body: Body,
        shard: u8,
    ) -> ProxyTunnelFuture {
        if shard < MAX_SHARDS {
            // Route to target shard
            match SHARD_REGISTER[shard as usize] {
                Some(ref shard_uri) => {
                    let mut tunnel_uri = format!(
                        "{}://{}{}",
                        shard_uri
                            .scheme_part()
                            .map(|scheme| scheme.as_str())
                            .unwrap_or("http"),
                        shard_uri
                            .authority_part()
                            .map(|authority| authority.as_str())
                            .unwrap_or(""),
                        uri.path()
                    );

                    if let Some(query) = uri.query() {
                        tunnel_uri.push_str("?");
                        tunnel_uri.push_str(query);
                    }

                    match tunnel_uri.parse::<Uri>() {
                        Ok(tunnel_uri) => {
                            // Forward body?
                            // Notice: HTTP DELETE is not forbidden per-spec to hold a request \
                            //   body, even if it is not commonly used. Hence why we forward it.
                            let req_body = match method {
                                &Method::POST | &Method::PATCH | &Method::PUT | &Method::DELETE => {
                                    body
                                }
                                _ => Body::empty(),
                            };

                            let mut tunnel_req = Request::new(req_body);

                            // Forward URI and method
                            *tunnel_req.method_mut() = method.clone();
                            *tunnel_req.uri_mut() = tunnel_uri;

                            // Forward headers
                            *tunnel_req.headers_mut() = headers.clone();

                            TUNNEL_CLIENT.with(|client| {
                                Box::new(
                                    client
                                        .request(tunnel_req)
                                        .map_err(|err| -> ProxyError { Box::new(err) }),
                                ) as ProxyTunnelFuture
                            })
                        }
                        Err(_) => Box::new(future::err(Self::make_proxy_err("invalid tunnel uri"))),
                    }
                }
                None => Box::new(future::err(Self::make_proxy_err("shard not configured"))),
            }
        } else {
            // Shard out of bounds
            Box::new(future::err(Self::make_proxy_err("shard out of bounds")))
        }
    }

    fn make_proxy_err(msg: &'static str) -> ProxyError {
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, msg))
    }
}
