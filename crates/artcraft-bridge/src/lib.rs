use anyhow::{Context, Result, bail};
use praxilume_core::CommandEnvelope;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

pub const UPSTREAM_REPOS: &[&str] = &[
    "https://github.com/storytold/photocraft",
    "https://github.com/storytold/vectorcraft",
    "https://github.com/storytold/filmcraft",
    "https://github.com/storytold/lightcraft",
    "https://github.com/storytold/printcraft",
    "https://github.com/storytold/effectcraft",
    "https://github.com/storytold/designcraft",
];

#[derive(Debug, Clone)]
pub struct ControlEndpoint {
    pub host: String,
    pub port: u16,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
}

impl ControlEndpoint {
    pub fn localhost(port: u16) -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port,
            connect_timeout: Duration::from_secs(3),
            read_timeout: Duration::from_secs(10),
        }
    }

    pub fn send(&self, envelope: &CommandEnvelope, token: Option<&str>) -> Result<Value> {
        let token = token.context("an upstream control token is required for bridge calls")?;
        let addr = (self.host.as_str(), self.port)
            .to_socket_addrs()?
            .next()
            .context("could not resolve control endpoint")?;
        let mut stream = TcpStream::connect_timeout(&addr, self.connect_timeout)?;
        stream.set_read_timeout(Some(self.read_timeout))?;

        let mut wire = serde_json::to_value(envelope)?;
        wire.as_object_mut()
            .context("command envelope must serialize to an object")?
            .insert("control_token".to_owned(), Value::String(token.to_owned()));

        serde_json::to_writer(&mut stream, &wire)?;
        stream.write_all(b"\n")?;
        stream.flush()?;

        let mut response = String::new();
        BufReader::new(stream).read_line(&mut response)?;
        if response.trim().is_empty() {
            bail!("upstream returned an empty control response");
        }
        Ok(serde_json::from_str(response.trim()).context("upstream returned invalid JSON")?)
    }
}
