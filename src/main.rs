use crate::config::logs::init_logs;
use crate::config::properties::parse_properties;
use crate::dns::server::dns::DnsServer;
use crate::proxy::server::proxy::Proxy;
use crate::util::{Error, Result};
use log::error;
use tokio::signal;

mod config;
mod dns;
mod k8s;
mod proxy;
mod util;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let props = parse_properties()?;
    init_logs(&props.log_level);

    if props.dns.server.host.ne("") {
        // run dns server
        let dns = DnsServer::new(&props).await?;
        tokio::spawn(async {
            if let Err(e) = dns.serve().await {
                error!("Unable to serve dns server, error: {:?}", e)
            }
        });
    }

    if props.proxy.host.ne("") {
        // run proxy server
        let proxy = Proxy::new(&props).await?;
        tokio::spawn(async {
            if let Err(e) = proxy.serve().await {
                error!("Unable to serve proxy server, error: {:?}", e)
            }
        });
    }

    // wait for OS SIGTERM signal
    return match signal::ctrl_c().await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("Unable to handle shutdown signal, err: {:?}", e);
            Err(Error::try_from(e)?)
        }
    };
}
