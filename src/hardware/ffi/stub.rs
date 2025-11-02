use std::ffi::c_void;

// Provide platform-neutral stand-ins so the crate builds on non-Windows targets.

pub type HRESULT = i32;
pub type WORD = u16;
pub type DWORD = u32;
pub type LONG = i32;
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

const ERR_UNSUPPORTED: HRESULT = -1;

pub fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn init_imager_ipc(_index: u16) -> Result<(), HRESULT> {
    Err(ERR_UNSUPPORTED)
}

pub fn init_named_imager_ipc(_index: u16, _instance_name: &str) -> Result<(), HRESULT> {
    Err(ERR_UNSUPPORTED)
}

pub fn run_imager_ipc(_index: u16) -> Result<(), HRESULT> {
    Err(ERR_UNSUPPORTED)
}

pub fn start_imager_ipc(_index: u16) -> Result<(), HRESULT> {
    Err(ERR_UNSUPPORTED)
}

pub fn release_imager_ipc(_index: u16) -> Result<(), HRESULT> {
    Err(ERR_UNSUPPORTED)
}

pub fn close_application(_index: u16) {}

pub fn get_temp_chip(_index: u16) -> f32 {
    f32::NAN
}

pub fn get_temp_flag(_index: u16) -> f32 {
    f32::NAN
}

pub fn get_temp_box(_index: u16) -> f32 {
    f32::NAN
}

pub fn get_serial_number(_index: u16) -> u32 {
    0
}

pub fn set_flag(_index: u16, _value: bool) -> bool {
    false
}

pub fn get_flag(_index: u16) -> bool {
    false
}

pub fn reset_flag(_index: u16) {}

pub fn set_ipc_mode(_index: u16, _mode: u16) -> u16 {
    0
}

pub fn get_ipc_mode(_index: u16) -> u16 {
    0
}

pub fn get_frame_config(_index: u16) -> Result<(i32, i32, i32), HRESULT> {
    Err(ERR_UNSUPPORTED)
}

pub fn get_frame(
    _index: u16,
    _timeout: u16,
    _buffer: &mut [u8],
    _metadata: &mut FrameMetadata,
) -> Result<(), HRESULT> {
    Err(ERR_UNSUPPORTED)
}

pub fn set_log_file(_filename: &str, _log_level: i32, _append: bool) -> Result<(), HRESULT> {
    Err(ERR_UNSUPPORTED)
}
