use tokio::net::TcpStream;

pub(crate) type Error = Box<dyn std::error::Error + Send + Sync>;
pub(crate) type Result<T> = std::result::Result<T, Error>;

pub fn log_error_result(res: Result<()>) {
    match res {
        Ok(_) => {}
        Err(e) => {
            log::error!("{:?}", e)
        }
    }
}

pub async fn is_tls(stream: &TcpStream) -> Result<bool> {
    let mut handshake_buffer = [0u8; 3];
    stream.peek(handshake_buffer.as_mut_slice()).await?;
    // [0] 22 handshake
    // [1] 3 ssl
    // [2] 1 || 2 || 3 tls v[0 | 1 | (2, 3)]
    // [1..2] for tls v1.3 MAY also be 0x0301 for compatibility purposes
    let is_tls = handshake_buffer[0] == 22
        && handshake_buffer[1] == 3
        && (handshake_buffer[2] == 1 || handshake_buffer[2] == 3);

    Ok(is_tls)
}
