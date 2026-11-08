#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    println!("OmniConvert Core v{} initialized", omniconvert_lib::get_app_version());
}
