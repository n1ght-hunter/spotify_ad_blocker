#![windows_subsystem = "windows"]

pub mod ad_killer;

use ad_killer::spotify_add_killer;
use std::{
    mem::{MaybeUninit},
    sync::Arc,
};
use tokio::sync::Mutex;
use trayicon::{MenuBuilder, TrayIconBuilder};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{DispatchMessageA, GetMessageA, TranslateMessage},
};


#[tokio::main]
async fn main() {
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    enum Events {
        Exit,
    }
    let exit = Arc::new(Mutex::new(false));
    let proxy_exit = exit.clone();
    let spotify_proxy_exit = exit.clone();

    let (s, mut r) = tokio::sync::mpsc::channel::<Events>(100);

    let icon = include_bytes!("../assets/icon1.ico");

    // Needlessly complicated tray icon with all the whistles and bells
    let _tray_icon = TrayIconBuilder::new()
        .sender(s)
        .icon_from_buffer(icon)
        .tooltip("Exit Spotify ad blocker")
        .menu(MenuBuilder::new().item("Exit", Events::Exit))
        .build()
        .unwrap();

    tokio::spawn(spotify_add_killer(spotify_proxy_exit));

    tokio::spawn(async move {
        while let Some(event) = r.recv().await {
            println!("event");
            match event {
                Events::Exit => {
                    *proxy_exit.lock().await = true;
                }
            }
        }
    });

    loop {
        unsafe {
            if *exit.lock().await {
                break;
            }
            let mut msg = MaybeUninit::uninit();
            let bret = GetMessageA(msg.as_mut_ptr(), HWND::default(), 0, 0);
            if bret.as_bool() {
                TranslateMessage(msg.as_ptr());
                DispatchMessageA(msg.as_ptr());
            } else {
                break;
            }
        }
    }
}
