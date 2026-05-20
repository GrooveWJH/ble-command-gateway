use serde_json::Value;
use std::sync::Arc;

const TRACE_BOX_WIDTH: usize = 116;
const TRACE_BOX_TEXT_WIDTH: usize = TRACE_BOX_WIDTH - 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceWriteKind {
    Request,
    ChunkAck,
    EventAck,
}

impl TraceWriteKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::ChunkAck => "chunk-ack",
            Self::EventAck => "event-ack",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceEvent {
    TxRaw {
        kind: TraceWriteKind,
        bytes: Vec<u8>,
        redacted: bool,
    },
    QosTx {
        kind: TraceWriteKind,
        bytes: usize,
        write: &'static str,
    },
    QosFallback {
        kind: TraceWriteKind,
        reason: String,
    },
    RxRaw {
        bytes: Vec<u8>,
    },
    RxChunk {
        response_id: String,
        mode: String,
        index: u64,
        total: u64,
        payload: String,
    },
    RxFrame {
        response_id: String,
        cmd: Option<String>,
        code: String,
        phase: protocol::ResponsePhase,
        final_flag: bool,
        text: String,
    },
    RxAssembled {
        bytes: Vec<u8>,
    },
    QosChunkAck {
        response_id: String,
        response_seq: u64,
        chunk_index: usize,
        chunk_total: usize,
    },
    QosEventAck {
        response_id: String,
        response_seq: u64,
    },
    RequestRetry {
        request_id: String,
        attempt: usize,
        error: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceOptions {
    pub redact_secrets: bool,
}

impl TraceOptions {
    pub fn safe() -> Self {
        Self {
            redact_secrets: true,
        }
    }

    pub fn unsafe_raw() -> Self {
        Self {
            redact_secrets: false,
        }
    }
}

pub type TraceCallback = Arc<dyn Fn(TraceEvent) + Send + Sync>;

pub fn emit(trace: &Option<TraceCallback>, event: TraceEvent) {
    if let Some(trace) = trace {
        trace(event);
    }
}

pub fn printing_callback() -> TraceCallback {
    Arc::new(|event| println!("{}", format_trace_event(&event)))
}

pub fn format_trace_event(event: &TraceEvent) -> String {
    match event {
        TraceEvent::TxRaw {
            kind,
            bytes,
            redacted,
        } => format_trace_box(
            "[TX:raw]",
            &[
                format!("kind     {}", kind.label()),
                format!("bytes    {}", bytes.len()),
                format!("redacted {}", redacted),
                format!("packet   {}", String::from_utf8_lossy(bytes)),
            ],
        ),
        TraceEvent::QosTx { kind, bytes, write } => format_trace_box(
            "[QOS:tx]",
            &[
                format!("kind  {}", kind.label()),
                format!("bytes {}", bytes),
                format!("write {}", write),
            ],
        ),
        TraceEvent::QosFallback { kind, reason } => format_trace_box(
            "[QOS:tx]",
            &[
                format!("kind     {}", kind.label()),
                "fallback without-response".to_string(),
                format!("reason   {}", reason),
            ],
        ),
        TraceEvent::RxRaw { bytes } => format_trace_box(
            "[RX:raw]",
            &[
                format!("bytes  {}", bytes.len()),
                format!("packet {}", String::from_utf8_lossy(bytes)),
            ],
        ),
        TraceEvent::RxChunk {
            response_id,
            mode,
            index,
            total,
            payload,
        } => format_trace_box(
            "[RX:chunk]",
            &[
                format!("id            {}", response_id),
                format!("mode          {}", mode),
                format!("index         {index}/{total}"),
                format!("payload_bytes {}", payload.len()),
                format!("payload       {}", payload),
            ],
        ),
        TraceEvent::RxFrame {
            response_id,
            cmd,
            code,
            phase,
            final_flag,
            text,
        } => format_trace_box(
            "[RX:frame]",
            &[
                format!("id     {}", response_id),
                format!(
                    "cmd    {}",
                    cmd.clone().unwrap_or_else(|| "<missing>".to_string())
                ),
                format!("code   {}", code),
                format!("phase  {phase:?}"),
                format!("final  {}", final_flag),
                format!("text   {}", text),
            ],
        ),
        TraceEvent::RxAssembled { bytes } => format_trace_box(
            "[RX:assembled]",
            &[
                format!("bytes  {}", bytes.len()),
                format!("packet {}", String::from_utf8_lossy(bytes)),
            ],
        ),
        TraceEvent::QosChunkAck {
            response_id,
            response_seq,
            chunk_index,
            chunk_total,
        } => format_trace_box(
            "[QOS:ack]",
            &[
                format!("id    {}", response_id),
                format!("seq   {}", response_seq),
                format!("chunk {chunk_index}/{chunk_total}"),
            ],
        ),
        TraceEvent::QosEventAck {
            response_id,
            response_seq,
        } => format_trace_box(
            "[QOS:event-ack]",
            &[
                format!("id  {}", response_id),
                format!("seq {}", response_seq),
            ],
        ),
        TraceEvent::RequestRetry {
            request_id,
            attempt,
            error,
        } => format_trace_box(
            "[QOS:retry]",
            &[
                format!("id      {}", request_id),
                format!("attempt {}", attempt),
                format!("error   {}", error),
            ],
        ),
    }
}

fn format_trace_box(title: &str, lines: &[String]) -> String {
    let mut rendered = Vec::new();
    rendered.push(trace_box_top(title));
    for line in lines {
        for wrapped in wrap_chars(line, TRACE_BOX_TEXT_WIDTH) {
            rendered.push(trace_box_line(&wrapped));
        }
    }
    rendered.push(trace_box_bottom());
    rendered.join("\n")
}

fn trace_box_top(title: &str) -> String {
    let title = format!(" {title} ");
    let title_width = title.chars().count();
    if title_width >= TRACE_BOX_WIDTH {
        return format!("╭{}╮", trim_chars(&title, TRACE_BOX_WIDTH));
    }
    format!("╭{title}{}╮", "─".repeat(TRACE_BOX_WIDTH - title_width))
}

fn trace_box_line(line: &str) -> String {
    let line = trim_chars(line, TRACE_BOX_TEXT_WIDTH);
    let line_width = line.chars().count();
    format!(
        "│ {line}{} │",
        " ".repeat(TRACE_BOX_TEXT_WIDTH.saturating_sub(line_width))
    )
}

fn trace_box_bottom() -> String {
    format!("╰{}╯", "─".repeat(TRACE_BOX_WIDTH))
}

fn wrap_chars(line: &str, width: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for ch in line.chars() {
        if current.chars().count() == width {
            lines.push(current);
            current = String::new();
        }
        current.push(ch);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn trim_chars(line: &str, width: usize) -> String {
    line.chars().take(width).collect()
}

pub fn redacted_payload(bytes: &[u8]) -> (Vec<u8>, bool) {
    let Ok(mut value) = serde_json::from_slice::<Value>(bytes) else {
        return (bytes.to_vec(), false);
    };
    let changed = redact_value(&mut value);
    if !changed {
        return (bytes.to_vec(), false);
    }
    match serde_json::to_vec(&value) {
        Ok(bytes) => (bytes, true),
        Err(_) => (bytes.to_vec(), false),
    }
}

pub fn response_trace_events(raw: &[u8]) -> Vec<TraceEvent> {
    let mut events = vec![TraceEvent::RxRaw {
        bytes: raw.to_vec(),
    }];
    match protocol::parse_response(raw) {
        Ok(response) => {
            if let Some(chunk) = response
                .data
                .as_ref()
                .and_then(|data| data.get("chunk"))
                .and_then(|value| value.as_object())
            {
                events.push(TraceEvent::RxChunk {
                    response_id: response.id,
                    mode: chunk
                        .get("mode")
                        .and_then(Value::as_str)
                        .unwrap_or("<missing>")
                        .to_string(),
                    index: chunk.get("index").and_then(Value::as_u64).unwrap_or(0),
                    total: chunk.get("total").and_then(Value::as_u64).unwrap_or(0),
                    payload: chunk
                        .get("payload")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                });
            } else {
                events.push(TraceEvent::RxFrame {
                    response_id: response.id,
                    cmd: response.cmd,
                    code: response.code,
                    phase: response.phase,
                    final_flag: response.final_flag,
                    text: response.text,
                });
            }
        }
        Err(err) => {
            events.push(TraceEvent::RxFrame {
                response_id: "<parse-error>".to_string(),
                cmd: None,
                code: "BAD_JSON".to_string(),
                phase: protocol::ResponsePhase::Result,
                final_flag: true,
                text: err.to_string(),
            });
        }
    }
    events
}

fn redact_value(value: &mut Value) -> bool {
    match value {
        Value::Object(map) => {
            let mut changed = false;
            for (key, value) in map.iter_mut() {
                if is_secret_key(key) {
                    if !matches!(value, Value::String(text) if text == "***") {
                        *value = Value::String("***".to_string());
                        changed = true;
                    }
                } else {
                    changed |= redact_value(value);
                }
            }
            changed
        }
        Value::Array(values) => values.iter_mut().any(redact_value),
        _ => false,
    }
}

fn is_secret_key(key: &str) -> bool {
    matches!(
        key.to_ascii_lowercase().as_str(),
        "pwd" | "password" | "passphrase" | "psk"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_known_secret_keys() {
        let raw = br#"{"cmd":"wifi.provision","args":{"ssid":"Lab","pwd":"secret","password":"secret2","nested":{"psk":"abc"}}}"#;

        let (redacted, changed) = redacted_payload(raw);
        let text = String::from_utf8(redacted).unwrap();

        assert!(changed);
        assert!(text.contains(r#""pwd":"***""#));
        assert!(text.contains(r#""password":"***""#));
        assert!(text.contains(r#""psk":"***""#));
        assert!(!text.contains("secret"));
        assert!(text.contains("Lab"));
    }

    #[test]
    fn leaves_non_secret_payload_unchanged() {
        let raw = br#"{"cmd":"system.status","args":{}}"#;

        let (redacted, changed) = redacted_payload(raw);

        assert!(!changed);
        assert_eq!(redacted, raw);
    }

    #[test]
    fn formats_rx_chunk_event() {
        let response = protocol::CommandResponse::ok("req", "x".repeat(500), None);
        let chunk = protocol::chunking::chunk_response(response)
            .into_iter()
            .next()
            .unwrap();
        let raw = protocol::encode_response(&chunk).unwrap();

        let events = response_trace_events(&raw);
        let formatted = events.iter().map(format_trace_event).collect::<Vec<_>>();

        assert!(formatted.iter().any(|line| line.contains("[RX:raw]")));
        assert!(formatted.iter().any(|line| line.contains("[RX:chunk]")));
    }

    #[test]
    fn formats_tx_qos_and_assembled_events() {
        let tx = format_trace_event(&TraceEvent::TxRaw {
            kind: TraceWriteKind::Request,
            bytes: br#"{"cmd":"system.status"}"#.to_vec(),
            redacted: false,
        });
        let qos = format_trace_event(&TraceEvent::QosChunkAck {
            response_id: "req".to_string(),
            response_seq: 2,
            chunk_index: 1,
            chunk_total: 3,
        });
        let assembled = format_trace_event(&TraceEvent::RxAssembled {
            bytes: br#"{"ok":true}"#.to_vec(),
        });

        assert!(tx.contains("[TX:raw]"));
        assert!(tx.contains("packet"));
        assert!(qos.contains("[QOS:ack]"));
        assert!(assembled.contains("[RX:assembled]"));
        assert!(assembled.contains("packet"));
    }

    #[test]
    fn formats_each_trace_event_as_a_single_box() {
        let formatted = format_trace_event(&TraceEvent::TxRaw {
            kind: TraceWriteKind::Request,
            bytes: br#"{"cmd":"system.status"}"#.to_vec(),
            redacted: false,
        });

        assert!(formatted.starts_with("╭"));
        assert!(formatted.contains("[TX:raw]"));
        assert!(formatted.contains("kind     request"));
        assert!(formatted.contains("packet   {\"cmd\":\"system.status\"}"));
        assert!(formatted.ends_with("╯"));
    }
}
