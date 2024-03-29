use anyhow::anyhow;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::str::FromStr;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;

pub fn log_error_result(res: anyhow::Result<()>) {
    match res {
        Ok(_) => {}
        Err(e) => {
            log::error!("{:?}", e)
        }
    }
}

pub async fn is_tls(stream: &TcpStream) -> anyhow::Result<bool> {
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

pub async fn load_local_cache(path: &String) -> anyhow::Result<HashMap<String, SocketAddr>> {
    let mut lines = read_lines(path).await?;

    let lines: HashMap<String, SocketAddr> = lines
        .iter_mut()
        .map(|line| line.split_once("="))
        .filter(|value| value.is_some())
        .map(|value| value.unwrap())
        .map(|(url, ip)| {
            let addr = match SocketAddr::from_str(ip) {
                Ok(addr) => Ok(addr),
                Err(_) => {
                    // parse without port
                    match IpAddr::from_str(ip) {
                        Ok(ip_addr) => Ok(SocketAddr::new(ip_addr, 0)),
                        Err(_) => Err("Unknown ip")
                    }
                }
            };
            (url, addr)
        })
        .filter(|(_, ip)| ip.is_ok())
        .map(|(url, ip)| (url, ip.unwrap()))
        .map(|(url, ip)| {
            return (url.to_string(), ip);
        })
        .collect();

    return Ok(lines);
}

async fn read_lines<P>(filename: P) -> anyhow::Result<Vec<String>>
where
    P: AsRef<Path>,
{
    let file = match File::open(filename).await {
        Ok(f) => Ok(f),
        Err(e) => Err(anyhow!("Can't open file, err: {:#?}", e)),
    }?;
    let mut line_buf = BufReader::new(file).lines();
    let mut lines: Vec<String> = Vec::default();

    while let Some(line) = line_buf.next_line().await? {
        lines.push(line);
    }

    return Ok(lines);
}
