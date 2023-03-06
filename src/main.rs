#![windows_subsystem = "windows"]

pub mod ad_killer;

use ad_killer::spotify_add_killer;
use log::{debug, info, LevelFilter};
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};
use std::{mem::MaybeUninit, sync::Arc};
use tokio::sync::Mutex;
use trayicon::{MenuBuilder, TrayIconBuilder};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{DispatchMessageA, GetMessageA, TranslateMessage},
};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Events {
    Exit,
}

#[derive(Clone)]
struct EventHanler<T>(tokio::sync::mpsc::Sender<T>);

impl<T: Clone> trayicon::Sender<T> for EventHanler<T> {
    fn send(&self, event: T) {
        self.0.try_send(event).map_err(|_| "error sending").unwrap();
    }
}

#[tokio::main]
async fn main() {
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{l} {d(%Y-%m-%d %H:%M:%S)(local)} - {m}\n",
        )))
        .build("log/output.log")
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(
            Root::builder()
                .appender("logfile")
                .build(LevelFilter::Debug),
        )
        .unwrap();

    log4rs::init_config(config).unwrap();

    info!("starting spotify ad blocker");
    let exit = Arc::new(Mutex::new(false));
    let proxy_exit = exit.clone();
    let spotify_proxy_exit = exit.clone();

    let (s, mut r) = tokio::sync::mpsc::channel::<Events>(100);

    let icon = include_bytes!("../assets/icon1.ico");

    debug!("setting up tray icon");
    // Needlessly complicated tray icon with all the whistles and bells
    let _tray_icon = TrayIconBuilder::new()
        .sender(EventHanler(s))
        .icon_from_buffer(icon)
        .tooltip("Exit Spotify ad blocker")
        .menu(MenuBuilder::new().item("Exit", Events::Exit))
        .build()
        .unwrap();

    info!("spawning spodify add killer");
    tokio::spawn(spotify_add_killer(spotify_proxy_exit));

    info!("spawning event handler");
    tokio::spawn(async move {
        while let Some(event) = r.recv().await {
            match event {
                Events::Exit => {
                    info!("event {:?}", event);
                    *proxy_exit.lock().await = true;
                }
            }
        }
    });

    info!("start system loop");
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
