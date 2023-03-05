use std::{ffi::OsString, os::windows::prelude::OsStringExt, sync::Arc, time::Duration};

use tokio::sync::Mutex;
use windows::{
    Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    },
    Win32::{
        Foundation::{
            self, CloseHandle, GetLastError, BOOL, HANDLE, HINSTANCE, HWND, LPARAM, WIN32_ERROR,
        },
        System::{
            ProcessStatus::{K32GetModuleBaseNameW, K32GetModuleFileNameExW},
            Threading::{
                OpenProcess, TerminateProcess, PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE,
                PROCESS_VM_READ,
            },
        },
        UI::WindowsAndMessaging::{
            EnumWindows, GetWindow, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
            GW_OWNER,
        },
    },
};

pub async fn spotify_add_killer(exit: Arc<Mutex<bool>>) {
    let mut data = get_data(exit.clone()).await;

    loop {
        if *exit.lock().await {
            break;
        }
        let session = {
            let mut ses = get_spotify_sesstion().await;
            while ses.is_none() {
                ses = get_spotify_sesstion().await;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            ses.unwrap()
        };
        if !data.title.contains("-")
            && session.GetPlaybackInfo().unwrap().PlaybackStatus().unwrap()
                == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
        {
            unsafe {
                let handle = OpenProcess(PROCESS_TERMINATE, false, data.pid).unwrap();
                TerminateProcess(handle, 1);
                CloseHandle(handle);
                tokio::time::sleep(Duration::from_millis(200)).await;
                tokio::process::Command::new(data.exe_path.clone())
                    .args(["--minimized"])
                    .spawn()
                    .unwrap().wait().await.unwrap();
                tokio::time::sleep(Duration::from_millis(200)).await;
                let session = {
                    let mut ses = get_spotify_sesstion().await;
                    while ses.is_none() {
                        ses = get_spotify_sesstion().await;
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    ses.unwrap()
                };
                while session.GetPlaybackInfo().unwrap().PlaybackStatus().unwrap()
                    != GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
                {
                    session.TrySkipNextAsync().unwrap().await.unwrap();
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
                data = get_data(exit.clone()).await;
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        } else {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            data.title = get_title(data.hwnd);
        }
    }
}

async fn get_data(exit: Arc<Mutex<bool>>) -> Box<Data> {
    let mut data: Option<Box<Data>> = None;

    while data.is_none() {
        if *exit.lock().await {
            break;
        }
        data = get_process();
        if data.is_none() {
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
    }
    data.unwrap()
}

#[derive(Debug)]

pub struct Data {
    pid: u32,
    title: String,
    hwnd: HWND,
    exe_path: String,
}

pub fn get_process() -> Option<Box<Data>> {
    unsafe {
        let mut data: Box<Data> = Box::new(Data {
            pid: 0,
            title: "".to_string(),
            hwnd: HWND::default(),
            exe_path: "".to_string(),
        });
        let handle_ptr: *mut Data = &mut *data;
        let state = LPARAM(handle_ptr as isize);
        EnumWindows(Some(window_enumer), state);
        if data.pid != 0 {
            Some(data)
        } else {
            None
        }
    }
}

async fn get_spotify_sesstion() -> Option<GlobalSystemMediaTransportControlsSession> {
    if let Ok(session_manager) = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .unwrap()
        .await
    {
        let sessions = session_manager.GetSessions();
        if let Ok(sessions) = sessions {
            let items = sessions.First().unwrap();
            for item in items {
                let name = item.SourceAppUserModelId().unwrap().to_string();
                if &name == "Spotify.exe" {
                    return Some(item);
                }
            }
        }
    }
    None
}

fn get_title(window: Foundation::HWND) -> String {
    unsafe {
        let mut exe_buf = [0u16; 256];
        GetWindowTextW(window, &mut exe_buf);
        null_terminated_wchar_to_string(&exe_buf)
    }
}

unsafe extern "system" fn window_enumer(
    window: Foundation::HWND,
    lparam: Foundation::LPARAM,
) -> Foundation::BOOL {
    let pid = Box::into_raw(Box::new(u32::default()));
    GetWindowThreadProcessId(window, Some(pid));
    let pid = *pid;
    let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
        .map_err(|x| format!("error: {}", x));
    if let Ok(process) = process {
        let process_name = get_process_name(process).unwrap();
        if process_name == "Spotify.exe"
            && GetWindow(window, GW_OWNER) == HWND(0)
            && IsWindowVisible(window).as_bool()
        {
            let exe_path = get_exe_path(process).unwrap();
            let window_title = get_title(window);
            let mut data: &mut Data = &mut *(lparam.0 as *mut Data);
            data.hwnd = window;
            data.pid = pid;
            data.title = window_title;
            data.exe_path = exe_path;
            return BOOL::from(false);
        }

        CloseHandle(process);
    }

    BOOL::from(true)
}

pub unsafe fn get_process_name(process: HANDLE) -> Result<String, WIN32_ERROR> {
    let mut exe_buf = [0u16; 256];
    if K32GetModuleBaseNameW(process, HINSTANCE::default(), &mut exe_buf) > 0 {
        Ok(null_terminated_wchar_to_string(&exe_buf))
    } else {
        Err(GetLastError())
    }
}

pub unsafe fn get_exe_path(process: HANDLE) -> Result<String, WIN32_ERROR> {
    let mut exe_buf = [0u16; 256];
    if K32GetModuleFileNameExW(process, HINSTANCE::default(), &mut exe_buf) > 0 {
        Ok(null_terminated_wchar_to_string(&exe_buf))
    } else {
        Err(GetLastError())
    }
}

pub unsafe fn null_terminated_wchar_to_string(slice: &[u16]) -> String {
    match slice.iter().position(|&x| x == 0) {
        Some(pos) => OsString::from_wide(&slice[..pos])
            .to_string_lossy()
            .into_owned(),
        None => OsString::from_wide(slice).to_string_lossy().into_owned(),
    }
}
