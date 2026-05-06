// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use std::convert::Infallible;

use hyper::service::make_service_fn;
use hyper::Server;
use tokio::runtime::Runtime;

use super::handle::ServerRequestHandle;
use crate::APP_CONF;

pub struct ServerListenBuilder;
pub struct ServerListen;

impl ServerListenBuilder {
    pub fn new() -> ServerListen {
        ServerListen {}
    }
}

impl ServerListen {
    pub fn run(&self) {
        let server_inet = APP_CONF.server.inet;

        Runtime::new()
            .expect("failed to create server runtime")
            .block_on(async {
                let service =
                    make_service_fn(|_conn| async { Ok::<_, Infallible>(ServerRequestHandle) });

                let server = Server::bind(&server_inet).serve(service);

                info!("listening on http://{}", server_inet);

                if let Err(err) = server.await {
                    error!("server general error: {}", err);
                }
            });
    }
}
