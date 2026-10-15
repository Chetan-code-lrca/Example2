use anyhow::{Context, Result, bail};
use praxilume_core::CommandEnvelope;
use praxilume_image_capability::upstream_command_id;
use serde_json::{Value, json};
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
            // PhotoCraft's desktop control server allows up to 60 seconds for a reply.
            read_timeout: Duration::from_secs(65),
        }
    }

    /// Send a Praxilume engine command through PhotoCraft's authenticated control protocol.
    pub fn send(&self, envelope: &CommandEnvelope, token: Option<&str>) -> Result<Value> {
        if envelope.dry_run {
            return Ok(json!({
                "dry_run": true,
                "protocol": envelope.protocol,
                "command_id": envelope.command_id.as_str(),
                "command": envelope.command,
                "params": envelope.params,
            }));
        }

        validate_engine_params(&envelope.params)?;
        let frame = command_frame(envelope);
        self.send_request(
            envelope.command_id.as_str(),
            &frame,
            token,
            &format!("command '{}'", envelope.command),
        )
    }

    /// Open a file under PhotoCraft's configured automation read root.
    ///
    /// The desktop control protocol applies the root capability. Praxilume also rejects
    /// absolute and traversal paths before sending the request.
    pub fn open_document(&self, relative_path: &str, token: Option<&str>) -> Result<Value> {
        validate_relative_path(relative_path)?;
        let frame = control_method_frame(
            "praxilume-open",
            "app.open",
            json!({ "path": relative_path }),
        );
        self.send_request("praxilume-open", &frame, token, "app.open")
    }

    /// Export/save an open document under PhotoCraft's configured automation write root.
    ///
    /// The parent output directory must already exist. Use a dedicated write root for exports.
    pub fn save_document(&self, relative_path: &str, token: Option<&str>) -> Result<Value> {
        validate_relative_path(relative_path)?;
        let frame = control_method_frame(
            "praxilume-save",
            "app.save",
            json!({ "path": relative_path }),
        );
        self.send_request("praxilume-save", &frame, token, "app.save")
    }

    fn send_request(
        &self,
        request_id: &str,
        request: &Value,
        token: Option<&str>,
        phase: &str,
    ) -> Result<Value> {
        let token = token.context("a PhotoCraft control token is required for bridge calls")?;
        if token.trim().is_empty() {
            bail!("a non-empty PhotoCraft control token is required for bridge calls");
        }

        let addr = (self.host.as_str(), self.port)
            .to_socket_addrs()?
            .next()
            .context("could not resolve control endpoint")?;
        let mut stream = TcpStream::connect_timeout(&addr, self.connect_timeout)?;
        stream.set_read_timeout(Some(self.read_timeout))?;
        stream.set_write_timeout(Some(self.connect_timeout))?;
        let mut reader = BufReader::new(stream.try_clone()?);

        // PhotoCraft requires authentication as the first frame on each TCP connection.
        write_json_line(&mut stream, &authentication_frame(token))?;
        let auth_response = read_json_line(&mut reader, "authentication")?;
        if auth_response.get("id") != Some(&json!("auth")) {
            bail!("PhotoCraft returned an unexpected authentication response ID");
        }
        ensure_success(&auth_response, "authentication")?;

        write_json_line(&mut stream, request)?;
        let response = read_json_line(&mut reader, phase)?;
        if response.get("id") != Some(&json!(request_id)) {
            bail!("PhotoCraft control response ID did not match the request");
        }
        ensure_success(&response, phase)?;
        Ok(response)
    }
}

fn authentication_frame(token: &str) -> Value {
    json!({
        "id": "auth",
        "method": "auth",
        "params": { "token": token },
    })
}

fn command_frame(envelope: &CommandEnvelope) -> Value {
    let command = match upstream_command_id(envelope.command.as_str()) {
        Some(command) => command,
        None => envelope.command.as_str(),
    };
    json!({
        "id": envelope.command_id.as_str(),
        "method": "engine.execute",
        "params": {
            "command": command,
            "params": envelope.params,
        },
    })
}

fn control_method_frame(id: &str, method: &str, params: Value) -> Value {
    json!({
        "id": id,
        "method": method,
        "params": params,
    })
}

fn validate_relative_path(path: &str) -> Result<()> {
    let invalid = path.is_empty()
        || path.trim() != path
        || path.contains('\0')
        || path.contains('\\')
        || path.contains(':')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..");
    if invalid {
        bail!(
            "path must be normalized and relative to PhotoCraft's configured automation root; absolute paths, '.', '..', backslashes, drive prefixes, and empty path segments are not allowed"
        );
    }
    Ok(())
}

fn validate_engine_params(params: &Value) -> Result<()> {
    if !params.is_object() && !params.is_null() {
        bail!("PhotoCraft engine.execute requires command params to be a JSON object or null");
    }
    Ok(())
}

fn ensure_success(response: &Value, phase: &str) -> Result<()> {
    match response.get("ok").and_then(Value::as_bool) {
        Some(true) => Ok(()),
        Some(false) => {
            let detail = response
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("no error detail");
            bail!("PhotoCraft {phase} failed: {detail}");
        }
        None => bail!("PhotoCraft returned an invalid {phase} response (expected boolean 'ok')"),
    }
}

fn write_json_line(stream: &mut TcpStream, value: &Value) -> Result<()> {
    serde_json::to_writer(&mut *stream, value)?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(())
}

fn read_json_line(reader: &mut BufReader<TcpStream>, phase: &str) -> Result<Value> {
    let mut line = String::new();
    let bytes = reader
        .read_line(&mut line)
        .with_context(|| format!("failed while reading PhotoCraft {phase} response"))?;
    if bytes == 0 || line.trim().is_empty() {
        bail!("PhotoCraft closed the connection without a {phase} response");
    }
    serde_json::from_str(line.trim())
        .with_context(|| format!("PhotoCraft returned invalid JSON for {phase}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use praxilume_core::{Actor, CommandEnvelope};

    fn test_envelope(params: Value) -> CommandEnvelope {
        CommandEnvelope::new(
            "cmd-7",
            Actor::User,
            "test image operation",
            "image.adjustments.invert",
            params,
        )
    }

    #[test]
    fn authentication_frame_matches_photocraft_protocol() {
        assert_eq!(
            authentication_frame("test-token"),
            json!({
                "id": "auth",
                "method": "auth",
                "params": { "token": "test-token" }
            })
        );
    }

    #[test]
    fn command_frame_wraps_command_in_engine_execute() {
        let frame = command_frame(&test_envelope(json!({ "enabled": true })));
        assert_eq!(
            frame,
            json!({
                "id": "cmd-7",
                "method": "engine.execute",
                "params": {
                    "command": "image.adjustments.invert",
                    "params": { "enabled": true }
                }
            })
        );
        assert!(frame.get("control_token").is_none());
        assert!(frame.get("protocol").is_none());
    }

    #[test]
    fn command_frame_translates_canonical_image_commands() {
        let mut envelope = test_envelope(json!({}));
        envelope.command = "image.invert".to_owned();
        let frame = command_frame(&envelope);
        assert_eq!(frame["params"]["command"], "image.adjustments.invert");
        assert_eq!(frame["params"]["params"], json!({}));
    }

    #[test]
    fn command_params_must_be_object_or_null() {
        assert!(validate_engine_params(&json!({})).is_ok());
        assert!(validate_engine_params(&Value::Null).is_ok());
        assert!(validate_engine_params(&json!(["wrong"])).is_err());
        assert!(validate_engine_params(&json!("wrong")).is_err());
    }

    #[test]
    fn control_method_frames_are_explicit_and_token_free() {
        assert_eq!(
            control_method_frame(
                "praxilume-open",
                "app.open",
                json!({"path": "assets/input.png"})
            ),
            json!({
                "id": "praxilume-open",
                "method": "app.open",
                "params": {"path": "assets/input.png"}
            })
        );
    }

    #[test]
    fn root_scoped_paths_reject_absolute_and_traversal_forms() {
        for path in [
            "",
            "/etc/passwd",
            "../outside.png",
            "assets/../outside.png",
            "assets//photo.png",
            "./assets/photo.png",
            r"C:\\Windows\\photo.png",
            "C:/Windows/photo.png",
            "assets/photo.png\\",
        ] {
            assert!(
                validate_relative_path(path).is_err(),
                "path accepted: {path:?}"
            );
        }
        assert!(validate_relative_path("assets/photo.png").is_ok());
        assert!(validate_relative_path("exports/final.png").is_ok());
    }

    #[test]
    fn upstream_errors_are_reported() {
        let err = ensure_success(
            &json!({ "id": "cmd-7", "ok": false, "error": "no document open" }),
            "command",
        )
        .expect_err("upstream error should fail");
        assert!(err.to_string().contains("no document open"));
    }
}
