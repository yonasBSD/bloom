// Bloom
//
// HTTP REST API caching middleware
// Copyright: 2017, Valerian Saliou <valerian@valeriansaliou.name>
// License: Mozilla Public License v2.0 (MPL v2.0)

use futures::Future;
use hyper::Server;

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

        let server = Server::bind(&server_inet)
            .serve(|| Ok::<_, hyper::Error>(ServerRequestHandle))
            .map_err(|err| error!("server error: {}", err));

        info!("listening on http://{}", server_inet);

        hyper::rt::run(server);
    }
}
