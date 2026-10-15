use anyhow::{Context, Result, bail};
use praxilume_core::CommandEnvelope;
use praxilume_image_capability::upstream_command_id as upstream_image_command_id;
use praxilume_vector_capability::upstream_command_id as upstream_vector_command_id;
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

/// VectorCraft uses an unauthenticated loopback control socket. Keep this adapter
/// pinned to 127.0.0.1 and configure VectorCraft's automation roots before use.
#[derive(Debug, Clone)]
pub struct VectorCraftEndpoint {
    port: u16,
    connect_timeout: Duration,
    read_timeout: Duration,
}

impl VectorCraftEndpoint {
    pub fn localhost(port: u16) -> Self {
        Self {
            port,
            connect_timeout: Duration::from_secs(3),
            read_timeout: Duration::from_secs(65),
        }
    }

    pub fn send(&self, envelope: &CommandEnvelope) -> Result<Value> {
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
        let request = vector_command_frame(envelope);
        self.send_request(
            envelope.command_id.as_str(),
            &request,
            &format!("command '{}'", envelope.command),
        )
    }

    /// Open a file beneath VectorCraft's configured automation read root.
    pub fn open_document(&self, relative_path: &str) -> Result<Value> {
        validate_relative_path(relative_path)?;
        self.send_method(
            "praxilume-vector-open",
            "app.open",
            json!({"path": relative_path}),
            "app.open",
        )
    }

    /// Export the active document beneath VectorCraft's configured automation write root.
    pub fn export_document(&self, relative_path: &str) -> Result<Value> {
        validate_relative_path(relative_path)?;
        self.send_method(
            "praxilume-vector-export",
            "app.export",
            json!({"path": relative_path}),
            "app.export",
        )
    }

    /// Query the live command catalog advertised by the running VectorCraft engine.
    pub fn list_commands(&self) -> Result<Value> {
        self.send_method(
            "praxilume-vector-commands",
            "engine.commands",
            json!({}),
            "engine.commands",
        )
    }

    /// Read the active document's structured layer/object tree without changing it.
    pub fn inspect_document(&self) -> Result<Value> {
        self.send_method(
            "praxilume-vector-inspect",
            "document.inspect",
            json!({}),
            "document.inspect",
        )
    }

    fn send_method(
        &self,
        request_id: &str,
        method: &str,
        params: Value,
        phase: &str,
    ) -> Result<Value> {
        self.send_request(
            request_id,
            &control_method_frame(request_id, method, params),
            phase,
        )
    }

    fn send_request(&self, request_id: &str, request: &Value, phase: &str) -> Result<Value> {
        let address = format!("127.0.0.1:{}", self.port)
            .to_socket_addrs()?
            .next()
            .context("could not resolve VectorCraft loopback endpoint")?;
        let mut stream = TcpStream::connect_timeout(&address, self.connect_timeout)?;
        stream.set_read_timeout(Some(self.read_timeout))?;
        stream.set_write_timeout(Some(self.connect_timeout))?;
        let mut reader = BufReader::new(stream.try_clone()?);

        // VectorCraft's control protocol has no authentication handshake. The socket is
        // therefore deliberately hard-coded to loopback, and upstream file access must be
        // constrained with --automation-read-root / --automation-write-root.
        write_json_line(&mut stream, request)?;
        let response = read_json_line(&mut reader, phase)?;
        if response.get("id") != Some(&json!(request_id)) {
            bail!("VectorCraft control response ID did not match the request");
        }
        ensure_vector_success(&response, phase)?;
        Ok(response)
    }
}

/// FilmCraft exposes an unauthenticated JSON-lines control socket, so the endpoint
/// is hard-coded to loopback. Enable FilmCraft control only while using it; never
/// expose its port to another host.
#[derive(Debug, Clone)]
pub struct FilmCraftEndpoint {
    port: u16,
    connect_timeout: Duration,
    read_timeout: Duration,
}

impl FilmCraftEndpoint {
    pub fn localhost(port: u16) -> Self {
        Self {
            port,
            connect_timeout: Duration::from_secs(3),
            read_timeout: Duration::from_secs(65),
        }
    }

    /// Run one FilmCraft engine command using its documented engine.execute method.
    pub fn execute(&self, command: &str, params: Value) -> Result<Value> {
        validate_engine_params(&params)?;
        self.send_method(
            "praxilume-filmcraft-execute",
            "engine.execute",
            json!({"command": command, "params": params}),
            &format!("command '{command}'"),
        )
    }

    /// Read the current engine command registry, including command IDs and parameters.
    pub fn list_commands(&self) -> Result<Value> {
        self.send_method(
            "praxilume-filmcraft-commands",
            "engine.commands",
            json!({}),
            "engine.commands",
        )
    }

    /// Read the active sequence without mutating the project.
    pub fn inspect_sequence(&self) -> Result<Value> {
        self.execute("sequence.inspect", json!({}))
    }

    fn send_method(
        &self,
        request_id: &str,
        method: &str,
        params: Value,
        phase: &str,
    ) -> Result<Value> {
        let address = format!("127.0.0.1:{}", self.port)
            .to_socket_addrs()?
            .next()
            .context("could not resolve FilmCraft loopback endpoint")?;
        let mut stream = TcpStream::connect_timeout(&address, self.connect_timeout)?;
        stream.set_read_timeout(Some(self.read_timeout))?;
        stream.set_write_timeout(Some(self.connect_timeout))?;
        let mut reader = BufReader::new(stream.try_clone()?);

        write_json_line(
            &mut stream,
            &control_method_frame(request_id, method, params),
        )?;
        let response = read_json_line(&mut reader, phase)?;
        if response.get("id") != Some(&json!(request_id)) {
            bail!("FilmCraft control response ID did not match the request");
        }
        ensure_filmcraft_success(&response, phase)?;
        Ok(response)
    }
}

fn vector_command_frame(envelope: &CommandEnvelope) -> Value {
    let command = match upstream_vector_command_id(envelope.command.as_str()) {
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

fn ensure_vector_success(response: &Value, phase: &str) -> Result<()> {
    match response.get("ok").and_then(Value::as_bool) {
        Some(true) => Ok(()),
        Some(false) => {
            let detail = response
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("no error detail");
            bail!("VectorCraft {phase} failed: {detail}");
        }
        None => bail!("VectorCraft returned an invalid {phase} response (expected boolean 'ok')"),
    }
}

fn ensure_filmcraft_success(response: &Value, phase: &str) -> Result<()> {
    match response.get("ok").and_then(Value::as_bool) {
        Some(true) => Ok(()),
        Some(false) => {
            let detail = response
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("no error detail");
            bail!("FilmCraft {phase} failed: {detail}");
        }
        None => bail!("FilmCraft returned an invalid {phase} response (expected boolean 'ok')"),
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
    let command = match upstream_image_command_id(envelope.command.as_str()) {
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
            "path must be normalized and relative to the configured automation root; absolute paths, '.', '..', backslashes, drive prefixes, and empty path segments are not allowed"
        );
    }
    Ok(())
}

fn validate_engine_params(params: &Value) -> Result<()> {
    if !params.is_object() && !params.is_null() {
        bail!("engine.execute requires command params to be a JSON object or null");
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
        .with_context(|| format!("failed while reading control-server {phase} response"))?;
    if bytes == 0 || line.trim().is_empty() {
        bail!("control server closed the connection without a {phase} response");
    }
    serde_json::from_str(line.trim())
        .with_context(|| format!("control server returned invalid JSON for {phase}"))
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
    fn vector_frame_maps_canonical_command_without_authentication() {
        let envelope = CommandEnvelope::new(
            "vector-1",
            Actor::User,
            "draw a rectangle",
            "vector.rectangle",
            json!({"x": 1.0, "y": 2.0, "width": 10.0, "height": 20.0}),
        );
        let frame = vector_command_frame(&envelope);
        assert_eq!(frame["id"], "vector-1");
        assert_eq!(frame["method"], "engine.execute");
        assert_eq!(frame["params"]["command"], "shape.rectangle");
        assert_eq!(frame["params"]["params"]["width"], 10.0);
        assert_ne!(frame["method"], "auth");
    }

    #[test]
    fn vectorcraft_upstream_errors_are_reported() {
        let err = ensure_vector_success(
            &json!({"id": "vector-1", "ok": false, "error": "no document open"}),
            "command",
        )
        .expect_err("upstream error should fail");
        assert!(err.to_string().contains("no document open"));
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
        assert_eq!(
            control_method_frame("praxilume-vector-commands", "engine.commands", json!({})),
            json!({
                "id": "praxilume-vector-commands",
                "method": "engine.commands",
                "params": {}
            })
        );
        assert_eq!(
            control_method_frame("praxilume-vector-inspect", "document.inspect", json!({})),
            json!({
                "id": "praxilume-vector-inspect",
                "method": "document.inspect",
                "params": {}
            })
        );
    }

    #[test]
    fn filmcraft_frames_match_documented_control_protocol() {
        assert_eq!(
            control_method_frame(
                "praxilume-filmcraft-commands",
                "engine.commands",
                json!({})
            ),
            json!({
                "id": "praxilume-filmcraft-commands",
                "method": "engine.commands",
                "params": {}
            })
        );
        assert_eq!(
            control_method_frame(
                "praxilume-filmcraft-execute",
                "engine.execute",
                json!({"command": "sequence.inspect", "params": {}})
            ),
            json!({
                "id": "praxilume-filmcraft-execute",
                "method": "engine.execute",
                "params": {"command": "sequence.inspect", "params": {}}
            })
        );
    }

    #[test]
    fn filmcraft_success_responses_are_validated() {
        assert!(ensure_filmcraft_success(
            &json!({"id":"request-1","ok":true,"result":{}}),
            "test"
        ).is_ok());
        let error = ensure_filmcraft_success(
            &json!({"id":"request-1","ok":false,"error":"no active sequence"}),
            "command"
        ).expect_err("failed response must be reported");
        assert!(error.to_string().contains("no active sequence"));
        assert!(ensure_filmcraft_success(&json!({"id":"request-1"}), "test").is_err());
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
