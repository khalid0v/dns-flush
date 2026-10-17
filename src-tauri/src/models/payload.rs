use serde::Serialize;

/// Public-IP observation, normalized so the frontend sees one shape.
#[derive(Clone, Debug, Serialize)]
pub struct PublicIpPayload {
    pub ok: bool,
    pub value: String,
}

/// Transient banner for live-monitor network changes.
#[derive(Clone, Debug, Serialize)]
pub struct ToastPayload {
    pub text: String,
}
