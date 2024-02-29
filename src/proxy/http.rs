use anyhow::anyhow;
use httparse::{Request, Status};
use log::error;
use tokio::net::TcpStream;

/// Return host header and read buffer
pub(crate) async fn get_host(tcp_stream: &mut TcpStream) -> anyhow::Result<String> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = Request::new(&mut headers);

    let mut data = [0u8; 4096];
    tcp_stream.peek(data.as_mut_slice()).await?;

    match req.parse(&data.as_slice()) {
        Ok(status) => match status {
            Status::Complete(_) => {}
            Status::Partial => {
                error!(
                    "Http header is not parsed completely, that means is bigger than {}",
                    data.len()
                )
            }
        },
        Err(e) => {
            error!("Cannot parse http header: {}", e)
        }
    };

    match req.headers.iter().find(|header| header.name == "Host") {
        None => Err(anyhow!("Host not found")),
        Some(host) => {
            let host_value = String::from_utf8_lossy(host.value).to_string();
            Ok(host_value)
        }
    }
}
