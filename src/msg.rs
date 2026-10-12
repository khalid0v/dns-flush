use crate::win::adapters::AdapterInfo;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogKind {
    Info,
    Ok,
    Warn,
    Err,
    Head,
}

pub enum Msg {
    Log(LogKind, String),
    Adapters(Vec<AdapterInfo>),
    Done,
}

pub type Tx = std::sync::mpsc::Sender<Msg>;

pub fn send(tx: &Tx, kind: LogKind, text: impl Into<String>) {
    let _ = tx.send(Msg::Log(kind, text.into()));
}

pub fn info(tx: &Tx, text: impl Into<String>) {
    send(tx, LogKind::Info, text);
}

pub fn ok(tx: &Tx, text: impl Into<String>) {
    send(tx, LogKind::Ok, text);
}

pub fn warn(tx: &Tx, text: impl Into<String>) {
    send(tx, LogKind::Warn, text);
}

pub fn err(tx: &Tx, text: impl Into<String>) {
    send(tx, LogKind::Err, text);
}

pub fn head(tx: &Tx, text: impl Into<String>) {
    send(tx, LogKind::Head, text);
}
