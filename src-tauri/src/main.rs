// Windows 发行版不弹出额外的控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    miyin_launcher::run();
}
