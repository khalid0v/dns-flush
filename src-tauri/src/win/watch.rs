//! Background, read-only status monitor.
//!
//! Uses `netwatcher` to be told when the OS reports an interface/address
//! change, then re-reads the adapter snapshot over WMI. A fallback poll keeps
//! DNS-only changes (which `netwatcher` does not report) and missed events
//! covered. This module performs no repairs and never touches egui.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::msg::{MonitorCmd, StatusMsg};

use super::{WMIConnection, adapters, public_ip};

const FALLBACK: Duration = Duration::from_secs(15);
const DEBOUNCE: Duration = Duration::from_millis(800);
const PUBLIC_IP_INTERVAL: Duration = Duration::from_secs(5 * 60);
const PUBLIC_IP_MIN_GAP: Duration = Duration::from_secs(60);
const IDLE_WAIT: Duration = Duration::from_secs(3600);

/// Internal event merged from the UI command channel and the OS watcher.
enum Tick {
    Cmd(MonitorCmd),
    Wake,
}

/// Start the monitor. Returns the command sender the UI uses to control it and
/// the receiver it polls for read-only observations. `notify` is called
/// whenever the UI should repaint to pick up new observations; it is supplied
/// by the caller so this module stays free of any UI dependency.
pub fn start<F>(notify: F) -> (Sender<MonitorCmd>, Receiver<StatusMsg>)
where
    F: Fn() + Send + 'static,
{
    let (cmd_tx, cmd_rx) = mpsc::channel::<MonitorCmd>();
    let (status_tx, status_rx) = mpsc::channel::<StatusMsg>();
    let (tick_tx, tick_rx) = mpsc::channel::<Tick>();

    let watcher_tx = tick_tx.clone();
    thread::spawn(move || watch_loop(watcher_tx));

    let bridge_tx = tick_tx.clone();
    thread::spawn(move || {
        while let Ok(cmd) = cmd_rx.recv() {
            if bridge_tx.send(Tick::Cmd(cmd)).is_err() {
                break;
            }
        }
    });

    drop(tick_tx);
    thread::spawn(move || snapshot_loop(tick_rx, status_tx, notify));

    (cmd_tx, status_rx)
}

fn watch_loop(tick_tx: Sender<Tick>) {
    let Ok(mut watcher) = netwatcher::watch_interfaces_blocking() else {
        return;
    };
    loop {
        let update = watcher.changed();
        if update.is_initial {
            continue;
        }
        if tick_tx.send(Tick::Wake).is_err() {
            break;
        }
    }
}

fn snapshot_loop<F>(tick_rx: Receiver<Tick>, tx: Sender<StatusMsg>, notify: F)
where
    F: Fn() + Send + 'static,
{
    let mut enabled = false;
    let mut connection: Option<WMIConnection> = None;
    let mut last_public_ip: Option<Instant> = None;

    loop {
        let timeout = if enabled { FALLBACK } else { IDLE_WAIT };
        match tick_rx.recv_timeout(timeout) {
            Ok(Tick::Cmd(MonitorCmd::SetEnabled(true))) => {
                enabled = true;
                send_snapshot(&tx, &mut connection);
                maybe_public_ip(&tx, &mut last_public_ip, Duration::ZERO);
                notify();
            }
            Ok(Tick::Cmd(MonitorCmd::SetEnabled(false))) => enabled = false,
            Ok(Tick::Cmd(MonitorCmd::RefreshNow)) => {
                send_snapshot(&tx, &mut connection);
                maybe_public_ip(&tx, &mut last_public_ip, Duration::ZERO);
                notify();
            }
            Ok(Tick::Wake) => {
                if enabled {
                    thread::sleep(DEBOUNCE);
                    send_snapshot(&tx, &mut connection);
                    maybe_public_ip(&tx, &mut last_public_ip, PUBLIC_IP_MIN_GAP);
                    notify();
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                send_snapshot(&tx, &mut connection);
                maybe_public_ip(&tx, &mut last_public_ip, PUBLIC_IP_INTERVAL);
                notify();
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

/// Read the current adapters and forward them. A failed read drops the cached
/// WMI connection so the next tick reconnects instead of using a stale one.
fn send_snapshot(tx: &Sender<StatusMsg>, connection: &mut Option<WMIConnection>) {
    if connection.is_none() {
        *connection = adapters::connect().ok();
    }
    let list = connection.as_ref().and_then(|con| adapters::list(con).ok());
    match list {
        Some(list) => {
            let _ = tx.send(StatusMsg::Adapters(list));
        }
        None => *connection = None,
    }
}

fn maybe_public_ip(tx: &Sender<StatusMsg>, last: &mut Option<Instant>, min_gap: Duration) {
    let due = last.is_none_or(|checked| checked.elapsed() >= min_gap);
    if !due {
        return;
    }
    let result = public_ip::lookup().map_err(|error| error.to_string());
    let _ = tx.send(StatusMsg::PublicIp(result));
    *last = Some(Instant::now());
}
