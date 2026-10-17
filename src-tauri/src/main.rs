#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Binary entry point: everything lives in the `trubo_lib` library crate so
//! the setup (commands, monitor, event loop) stays testable and reusable.

fn main() {
    trubo_lib::run();
}
