// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::Service;
use hyper::{Request, Response};

use crate::proxy::serve::{ProxyServe, ProxyServeError, ProxyServeResponseFuture};

pub struct ServerRequestHandle;

impl Service<Request<Incoming>> for ServerRequestHandle {
    type Response = Response<Full<Bytes>>;
    type Error = ProxyServeError;
    type Future = ProxyServeResponseFuture;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        debug!("called proxy serve");

        Box::pin(ProxyServe::handle(req))
    }
}
