use std::{mem::MaybeUninit, thread, time};
use tokio::sync::mpsc::channel;
use windows::Win32::{
    Foundation::HWND,
    UI::{
        Shell::{Shell_NotifyIconA, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NOTIFYICONDATAA},
        WindowsAndMessaging::{DispatchMessageA, GetMessageA, TranslateMessage},
    },
};

pub const WM_USER_TRAYICON: u32 = 0x400 + 1001;

#[tokio::main]
async fn main() {
    let tooltip = None;
    let mut icon_part = NOTIFYICONDATAA::default();
    static mut ICON_ID: u32 = 1000;
    unsafe {
        ICON_ID += 1;
    }
    if let Some(tooltip) = tooltip {
        icon_part.szTip = tooltip
    }
    icon_part.cbSize = std::mem::size_of::<NOTIFYICONDATAA>() as u32;
    icon_part.uID = unsafe { ICON_ID };
    icon_part.uCallbackMessage = WM_USER_TRAYICON;
    icon_part.hIcon = icon_part.hIcon;
    icon_part.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;

    let icon: *const NOTIFYICONDATAA = &icon_part;
    unsafe {
        let result = Shell_NotifyIconA(NIM_ADD, icon);
    }
}
