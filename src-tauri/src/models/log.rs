use serde::Serialize;

/// One activity-log line, streamed as a `log` event and collected as the
/// command's return value.
#[derive(Clone, Debug, Serialize)]
pub struct LogLine {
    pub kind: String,
    pub text: String,
}
