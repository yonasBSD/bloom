// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper::service::Service;
use hyper::{Body, Request, Response};

use crate::proxy::serve::{ProxyError, ProxyServe};

pub struct ServerRequestHandle;

impl Service<Request<Body>> for ServerRequestHandle {
    type Response = Response<Body>;
    type Error = ProxyError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        debug!("called proxy serve");

        Box::pin(ProxyServe::handle(req))
    }
}
