// Prevent the command-line window from appearing on Windows.
// In release builds this is required; in dev builds cargo still shows a console.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    diva_live_helper_app_lib::run();
}
