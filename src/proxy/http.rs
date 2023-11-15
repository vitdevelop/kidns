use crate::Result;
use httparse::{Request, Status};
use log::{debug, error};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};

/// Return host header and read buffer
pub(crate) async fn get_host<'a, T>(tcp_stream: &mut T) -> Result<(String, Vec<u8>)>
where T: AsyncRead + AsyncWrite + Unpin{
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = Request::new(&mut headers);

    let mut data = [0u8; 4096];
    tcp_stream.read(data.as_mut_slice()).await?;

    let mut header_data = data.to_vec();

    match req.parse(&data.as_slice()) {
        Ok(status) => match status {
            Status::Complete(size) => {
                header_data.truncate(size);
            }
            Status::Partial => {
                error!(
                    "Http header is not parsed completely, that means is bigger than {}",
                    data.len()
                )
            }
        },
        Err(e) => {
            error!("Cannot parse http header: {}\n{:?}", e, header_data.as_slice())
        }
    };

    debug!("{:?}", std::str::from_utf8(header_data.as_slice()));

    match req.headers.iter().find(|header| header.name == "Host") {
        None => Err("Host not found".into()),
        Some(host) => {
            let host_value = String::from_utf8_lossy(host.value).to_string();
            Ok((host_value, header_data))
        }
    }
}
