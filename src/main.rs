use log::error;
use crate::config::logs::init_logs;
use crate::config::properties::parse_properties;
use crate::util::{Error, Result};
use tokio::signal;
use crate::dns::server::dns::DnsServer;
use crate::proxy::server::proxy::Proxy;

mod util;
mod dns;
mod config;
mod proxy;

#[tokio::main]
async fn main() -> Result<()> {
    let props = parse_properties()?;
    init_logs();

    // run dns server
    let dns = DnsServer::new(&props);
    tokio::spawn(async {
        if let Err(e) = dns.serve().await {
            error!("Unable to serve dns server, error: {:?}", e)
        }
    });

    // run proxy server
    let proxy = Proxy::new(&props).await?;
    tokio::spawn(async {
        if let Err(e) = proxy.serve().await {
            error!("Unable to serve proxy server, error: {:?}", e)
        }
    });

    // wait for OS SIGTERM signal
    return match signal::ctrl_c().await {
        Ok(_) => { Ok(()) }
        Err(e) => {
            error!("Unable to handle shutdown signal, err: {:?}", e);
            Err(Error::try_from(e)?)
        }
    };
}