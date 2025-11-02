use std::ffi::OsStr;
use std::ffi::{c_long, c_void};
use std::os::windows::ffi::OsStrExt;

// Manually define Windows types that were causing issues with bindgen
pub type HRESULT = c_long;
pub type WORD = u16;
pub type DWORD = u32;
pub type LONG = c_long;
pub type BOOL = i32;
pub type HWND = *mut c_void;
pub type UINT = u32;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type USHORT = u16;
pub type ULONG = u32;
pub type PVOID = *mut c_void;
pub type HANDLE = PVOID;
pub type UCHAR = u8;
pub type FLOAT = f32;

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TFlagState {
    FsFlagOpen,
    FsFlagClose,
    FsFlagOpening,
    FsFlagClosing,
    FsError,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FrameMetadata {
    pub size: u16,
    pub counter: u32,
    pub counter_hw: u32,
    pub timestamp: i64,
    pub timestamp_media: i64,
    pub flag_state: TFlagState,
    pub temp_chip: f32,
    pub temp_flag: f32,
    pub temp_box: f32,
    pub pif_in: [u16; 2],
}

#[link(name = "ImagerIPC2x64")]
extern "C" {
    // Initialization and Lifecycle
    fn InitImagerIPC(index: WORD) -> HRESULT;
    fn InitNamedImagerIPC(index: WORD, instance_name: *const u16) -> HRESULT;
    fn RunImagerIPC(index: WORD) -> HRESULT;
    fn StartImagerIPC(index: WORD) -> HRESULT;
    fn ReleaseImagerIPC(index: WORD) -> HRESULT;
    fn CloseApplication(index: WORD);

    // Frame Acquisition
    fn GetFrameConfig(index: WORD, width: *mut i32, height: *mut i32, depth: *mut i32) -> HRESULT;
    fn GetVisibleFrameConfig(
        index: WORD,
        width: *mut i32,
        height: *mut i32,
        depth: *mut i32,
    ) -> HRESULT;
    fn GetFrame(
        index: WORD,
        timeout: WORD,
        buffer: *mut c_void,
        size: DWORD,
        metadata: *mut FrameMetadata,
    ) -> HRESULT;
    fn GetVisibleFrame(
        index: WORD,
        timeout: WORD,
        buffer: *mut c_void,
        size: DWORD,
        metadata: *mut FrameMetadata,
    ) -> HRESULT;
    fn AcknowledgeFrame(index: WORD) -> HRESULT;

    // Logging
    fn SetLogFile(filename: *const u16, log_level: i32, append: BOOL) -> HRESULT;
    fn SetLogging(log_groups: i32) -> HRESULT;
    fn Log(index: WORD, logstring: *const i8, log_level: i32) -> HRESULT;

    // Temperature and Device Info
    fn GetTempChip(index: WORD) -> f32;
    fn GetTempFlag(index: WORD) -> f32;
    fn GetTempBox(index: WORD) -> f32;
    fn GetSerialNumber(index: WORD) -> ULONG;

    // Control and Configuration
    fn SetFlag(index: WORD, value: bool) -> bool;
    fn GetFlag(index: WORD) -> bool;
    fn ResetFlag(index: WORD);
    fn SetIPCMode(index: WORD, value: USHORT) -> USHORT;
    fn GetIPCMode(index: WORD) -> USHORT;
}

// Helper function to convert Rust string to wide string for Windows API
pub fn to_wide_string(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

// Wrapper functions for easier usage
pub fn init_imager_ipc(index: u16) -> Result<(), HRESULT> {
    let result = unsafe { InitImagerIPC(index) };
    if result >= 0 {
        Ok(())
    } else {
        Err(result)
    }
}

pub fn init_named_imager_ipc(index: u16, instance_name: &str) -> Result<(), HRESULT> {
    let wide_name = to_wide_string(instance_name);
    let result = unsafe { InitNamedImagerIPC(index, wide_name.as_ptr()) };
    if result >= 0 {
        Ok(())
    } else {
        Err(result)
    }
}

pub fn run_imager_ipc(index: u16) -> Result<(), HRESULT> {
    let result = unsafe { RunImagerIPC(index) };
    if result >= 0 {
        Ok(())
    } else {
        Err(result)
    }
}

pub fn start_imager_ipc(index: u16) -> Result<(), HRESULT> {
    let result = unsafe { StartImagerIPC(index) };
    if result >= 0 {
        Ok(())
    } else {
        Err(result)
    }
}

pub fn release_imager_ipc(index: u16) -> Result<(), HRESULT> {
    let result = unsafe { ReleaseImagerIPC(index) };
    if result >= 0 {
        Ok(())
    } else {
        Err(result)
    }
}

pub fn close_application(index: u16) {
    unsafe { CloseApplication(index) };
}

pub fn get_temp_chip(index: u16) -> f32 {
    unsafe { GetTempChip(index) }
}

pub fn get_temp_flag(index: u16) -> f32 {
    unsafe { GetTempFlag(index) }
}

pub fn get_temp_box(index: u16) -> f32 {
    unsafe { GetTempBox(index) }
}

pub fn get_serial_number(index: u16) -> u32 {
    unsafe { GetSerialNumber(index) }
}

pub fn set_flag(index: u16, value: bool) -> bool {
    unsafe { SetFlag(index, value) }
}

pub fn get_flag(index: u16) -> bool {
    unsafe { GetFlag(index) }
}

pub fn reset_flag(index: u16) {
    unsafe { ResetFlag(index) };
}

pub fn set_ipc_mode(index: u16, mode: u16) -> u16 {
    unsafe { SetIPCMode(index, mode) }
}

pub fn get_ipc_mode(index: u16) -> u16 {
    unsafe { GetIPCMode(index) }
}

pub fn get_frame_config(index: u16) -> Result<(i32, i32, i32), HRESULT> {
    let mut width = 0i32;
    let mut height = 0i32;
    let mut depth = 0i32;

    let result = unsafe { GetFrameConfig(index, &mut width, &mut height, &mut depth) };

    if result >= 0 {
        Ok((width, height, depth))
    } else {
        Err(result)
    }
}

pub fn get_frame(
    index: u16,
    timeout: u16,
    buffer: &mut [u8],
    metadata: &mut FrameMetadata,
) -> Result<(), HRESULT> {
    let result = unsafe {
        GetFrame(
            index,
            timeout,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
            metadata,
        )
    };

    if result >= 0 {
        Ok(())
    } else {
        Err(result)
    }
}

pub fn set_log_file(filename: &str, log_level: i32, append: bool) -> Result<(), HRESULT> {
    let wide_filename = to_wide_string(filename);
    let result = unsafe { SetLogFile(wide_filename.as_ptr(), log_level, append as BOOL) };

    if result >= 0 {
        Ok(())
    } else {
        Err(result)
    }
}
