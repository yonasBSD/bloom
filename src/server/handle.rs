// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use hyper::service::Service;
use hyper::{Body, Request};

use crate::proxy::serve::{ProxyError, ProxyServe, ProxyServeResponseFuture};

pub struct ServerRequestHandle;

impl Service for ServerRequestHandle {
    type ReqBody = Body;
    type ResBody = Body;
    type Error = ProxyError;
    type Future = ProxyServeResponseFuture;

    fn call(&mut self, req: Request<Body>) -> ProxyServeResponseFuture {
        debug!("called proxy serve");

        ProxyServe::handle(req)
    }
}
