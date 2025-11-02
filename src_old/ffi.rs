use anyhow::Result;
use std::ffi::OsString;
use std::os::windows::ffi::OsStrExt;
use windows::Win32::System::LibraryLoader::{LoadLibraryW, GetProcAddress};
use windows::core::{PCSTR, PCWSTR};
use log::{debug, error, info};

// Define Windows types that match the C++ header
pub type HRESULT = i32;
pub type WORD = u16;
pub type USHORT = u16;
pub type ULONG = u32;
pub type UCHAR = u8;
pub type BOOL = windows::Win32::Foundation::BOOL; // Use the Windows type instead of defining as i32

// Windows types from windows-rs
use windows::Win32::Foundation::{HINSTANCE as HMODULE};

// RECT structure from Windows
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

// POINT structure from Windows
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct POINT {
    pub x: i32,
    pub y: i32,
}

// SIZE structure from Windows
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SIZE {
    pub cx: i32,
    pub cy: i32,
}

// Define the enums from ImagerIPC2.h
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TIPCMode {
    IpcColors = 0,
    IpcTemps = 1,
    IpcAdus = 2,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TFlagState {
    FsFlagOpen = 0,
    FsFlagClose = 1,
    FsFlagOpening = 2,
    FsFlagClosing = 3,
    FsError = 4,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TRotationMode {
    RmOff = 0,
    RmCw90 = 1,
    RmAcw90 = 2,
    RmCw180 = 3,
    RmCwh = 4,
    RmCwv = 5,
    RmAcwh = 6,
    RmAcwv = 7,
    RmUser = 8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TZoomMode {
    ZmOff = 0,
    ZmMax = 1,
    ZmUser = 2,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MeasureAreaShape {
    MasOff = 0,
    MasMp1x1 = 1,
    MasMp3x3 = 2,
    MasMp5x5 = 3,
    MasUserDefRect = 4,
    MasEllipse = 5,
    MasPolygon = 6,
    MasCurve = 7,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MeasureAreaMode {
    MamMin = 0,
    MamMax = 1,
    MamAvg = 2,
    MamDist = 3,
}

// Define the structs from ImagerIPC2.h
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FrameMetadata {
    pub size: u16,           // size of this structure
    pub counter: u32,        // frame counter
    pub counter_hw: u32,     // frame counter hardware
    pub timestamp: i64,      // time stamp in UNITS (10000000 per second)
    pub timestamp_media: i64,
    pub flag_state: TFlagState,
    pub temp_chip: f32,
    pub temp_flag: f32,
    pub temp_box: f32,
    pub pi_fin: [WORD; 2],   // PIFin[2]
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VideoFormat {
    pub width_ir: i32,
    pub height_ir: i32,
    pub framerate_ir: i32,
    pub width_visible: i32,
    pub height_visible: i32,
    pub framerate_visible: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IRArranging {
    pub rotation: TRotationMode,
    pub rotation_angle: f32,
    pub zoom: BOOL,
    pub zoom_rect: RECT,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MeasureArea {
    pub shape: MeasureAreaShape,
    pub mode: MeasureAreaMode,
    pub bind_to_temp_profile: BOOL,
    pub use_emissivity: BOOL,
    pub emissivity: f32,
    pub show_in_dig_disp_group: BOOL,
    pub dist_min: f32,
    pub dist_max: f32,
    pub location: POINT,
    pub size: SIZE,
    pub is_hot_spot: BOOL,
    pub is_cold_spot: BOOL,
}

// Define function pointer types
pub type FpOnServerStopped = Option<unsafe extern "system" fn(i32) -> HRESULT>;
pub type FpOnFrameInit = Option<unsafe extern "system" fn(i32, i32, i32) -> HRESULT>;
pub type FpOnNewFrame = Option<unsafe extern "system" fn(*mut i8, i32) -> HRESULT>;
pub type FpOnNewFrameEx = Option<unsafe extern "system" fn(*mut std::ffi::c_void, *mut FrameMetadata) -> HRESULT>;
pub type FpOnInitCompleted = Option<unsafe extern "system" fn() -> HRESULT>;
pub type FpOnConfigChanged = Option<unsafe extern "system" fn(i32) -> HRESULT>; // long reserved
pub type FpOnStringSend = Option<unsafe extern "system" fn(*mut u16) -> HRESULT>; // wchar_t *Path

// Constants from ImagerIPC2.h


// Logging groups

// Logging levels

// Define the FFI function signatures
#[repr(C)]
pub struct ImagerIPC2Bindings {
    // Initialization and control functions
    pub set_imager_ipc_count: unsafe extern "system" fn(WORD) -> HRESULT,
    pub init_imager_ipc: unsafe extern "system" fn(WORD) -> HRESULT,
    pub init_named_imager_ipc: unsafe extern "system" fn(WORD, *mut u16) -> HRESULT,
    pub run_imager_ipc: unsafe extern "system" fn(WORD) -> HRESULT,
    pub start_imager_ipc: unsafe extern "system" fn(WORD) -> HRESULT,
    pub release_imager_ipc: unsafe extern "system" fn(WORD) -> HRESULT,
    pub imager_ipc_process_messages: unsafe extern "system" fn(WORD) -> HRESULT,
    pub acknowledge_frame: unsafe extern "system" fn(WORD) -> HRESULT,
    
    // Frame functions
    pub get_frame_config: unsafe extern "system" fn(WORD, *mut i32, *mut i32, *mut i32) -> HRESULT,
    pub get_visible_frame_config: unsafe extern "system" fn(WORD, *mut i32, *mut i32, *mut i32) -> HRESULT,
    pub get_frame: unsafe extern "system" fn(WORD, WORD, *mut std::ffi::c_void, u32, *mut FrameMetadata) -> HRESULT,
    pub get_visible_frame: unsafe extern "system" fn(WORD, WORD, *mut std::ffi::c_void, u32, *mut FrameMetadata) -> HRESULT,
    
    // Logging functions
    pub set_log_file: unsafe extern "system" fn(*mut u16, i32, BOOL) -> HRESULT,
    pub set_logging: unsafe extern "system" fn(i32) -> HRESULT,
    pub log: unsafe extern "system" fn(WORD, *mut i8, i32) -> HRESULT,
    
    // Callback functions
    pub set_callback_on_server_stopped: unsafe extern "system" fn(WORD, FpOnServerStopped) -> HRESULT,
    pub set_callback_on_frame_init: unsafe extern "system" fn(WORD, FpOnFrameInit) -> HRESULT,
    pub set_callback_on_new_frame: unsafe extern "system" fn(WORD, FpOnNewFrame) -> HRESULT,
    pub set_callback_on_new_frame_ex: unsafe extern "system" fn(WORD, FpOnNewFrameEx) -> HRESULT,
    pub set_callback_on_visible_frame_init: unsafe extern "system" fn(WORD, FpOnFrameInit) -> HRESULT,
    pub set_callback_on_new_visible_frame: unsafe extern "system" fn(WORD, FpOnNewFrame) -> HRESULT,
    pub set_callback_on_new_visible_frame_ex: unsafe extern "system" fn(WORD, FpOnNewFrameEx) -> HRESULT,
    pub set_callback_on_init_completed: unsafe extern "system" fn(WORD, FpOnInitCompleted) -> HRESULT,
    pub set_callback_on_config_changed: unsafe extern "system" fn(WORD, FpOnConfigChanged) -> HRESULT,
    pub set_callback_on_file_command_ready: unsafe extern "system" fn(WORD, FpOnStringSend) -> HRESULT,
    pub set_callback_on_new_nmea_string: unsafe extern "system" fn(WORD, FpOnStringSend) -> HRESULT,
    
    // Get/Set procedures
    pub get_version_application: unsafe extern "system" fn(WORD) -> i64,
    pub get_version_hid_dll: unsafe extern "system" fn(WORD) -> i64,
    pub get_version_cd_dll: unsafe extern "system" fn(WORD) -> i64,
    pub get_version_ipc_dll: unsafe extern "system" fn(WORD) -> i64,
    
    pub get_temp_chip: unsafe extern "system" fn(WORD) -> f32,
    pub get_temp_flag: unsafe extern "system" fn(WORD) -> f32,
    pub get_temp_proc: unsafe extern "system" fn(WORD) -> f32,
    pub get_temp_box: unsafe extern "system" fn(WORD) -> f32,
    pub get_temp_housing: unsafe extern "system" fn(WORD) -> f32,
    pub get_temp_target: unsafe extern "system" fn(WORD) -> f32,
    pub get_humidity: unsafe extern "system" fn(WORD) -> f32,
    pub get_temp_range_count: unsafe extern "system" fn(WORD) -> USHORT,
    pub get_optics_count: unsafe extern "system" fn(WORD) -> USHORT,
    pub get_measure_area_count: unsafe extern "system" fn(WORD) -> USHORT,
    pub get_video_format_count: unsafe extern "system" fn(WORD) -> USHORT,
    pub get_clipped_format_max_pos: unsafe extern "system" fn(WORD, *mut POINT) -> HRESULT,
    pub get_temp_min_range: unsafe extern "system" fn(WORD, ULONG) -> f32,
    pub get_temp_max_range: unsafe extern "system" fn(WORD, ULONG) -> f32,
    pub get_optics_fov: unsafe extern "system" fn(WORD, ULONG) -> USHORT,
    pub get_optics_serial_number: unsafe extern "system" fn(WORD, ULONG) -> ULONG,
    pub get_temp_measure_area: unsafe extern "system" fn(WORD, ULONG) -> f32,
    pub get_loc_measure_area: unsafe extern "system" fn(WORD, ULONG, *mut POINT) -> HRESULT,
    pub set_loc_measure_area: unsafe extern "system" fn(WORD, ULONG, POINT) -> HRESULT,
    pub get_init_counter: unsafe extern "system" fn(WORD) -> USHORT,
    pub get_ipc_state: unsafe extern "system" fn(WORD, BOOL) -> USHORT,
    pub get_ipc_mode: unsafe extern "system" fn(WORD) -> USHORT,
    pub set_ipc_mode: unsafe extern "system" fn(WORD, USHORT) -> USHORT,
    pub get_frame_queue: unsafe extern "system" fn(WORD) -> USHORT,
    pub get_visible_frame_queue: unsafe extern "system" fn(WORD) -> USHORT,
    pub get_video_format: unsafe extern "system" fn(WORD, ULONG, *mut VideoFormat) -> HRESULT,
    pub get_ir_arranging: unsafe extern "system" fn(WORD, *mut IRArranging) -> HRESULT,
    pub set_ir_arranging: unsafe extern "system" fn(WORD, *mut IRArranging) -> HRESULT,
    pub get_path_of_stored_file: unsafe extern "system" fn(WORD, *mut u16, i32) -> HRESULT,
    pub get_new_nmea_string: unsafe extern "system" fn(WORD, *mut u16, i32) -> HRESULT,
    pub get_measure_area: unsafe extern "system" fn(WORD, ULONG, *mut MeasureArea) -> HRESULT,
    pub set_measure_area: unsafe extern "system" fn(WORD, ULONG, *mut MeasureArea, BOOL) -> HRESULT,
    pub remove_measure_area: unsafe extern "system" fn(WORD, ULONG) -> HRESULT,
    pub add_measure_area_point: unsafe extern "system" fn(WORD, ULONG, POINT) -> HRESULT,
    pub set_measure_area_name: unsafe extern "system" fn(WORD, ULONG, *mut u16) -> HRESULT,
    pub get_measure_area_name: unsafe extern "system" fn(WORD, ULONG, *mut u16, *mut i32, i32) -> HRESULT,
    
    // Flag and device control
    pub get_flag: unsafe extern "system" fn(WORD) -> BOOL,
    pub set_flag: unsafe extern "system" fn(WORD, BOOL) -> BOOL,
    pub get_optics_index: unsafe extern "system" fn(WORD) -> USHORT,
    pub set_optics_index: unsafe extern "system" fn(WORD, USHORT) -> USHORT,
    pub get_temp_range_index: unsafe extern "system" fn(WORD) -> USHORT,
    pub set_temp_range_index: unsafe extern "system" fn(WORD, USHORT) -> USHORT,
    pub get_video_format_index: unsafe extern "system" fn(WORD) -> USHORT,
    pub set_video_format_index: unsafe extern "system" fn(WORD, USHORT) -> USHORT,
    pub get_clipped_format_pos: unsafe extern "system" fn(WORD, *mut POINT) -> HRESULT,
    pub set_clipped_format_pos: unsafe extern "system" fn(WORD, POINT) -> HRESULT,
    pub get_temp_range_decimal: unsafe extern "system" fn(WORD, BOOL) -> USHORT,
    pub get_main_window_embedded: unsafe extern "system" fn(WORD) -> BOOL,
    pub set_main_window_embedded: unsafe extern "system" fn(WORD, BOOL) -> BOOL,
    pub get_main_window_loc_x: unsafe extern "system" fn(WORD) -> USHORT,
    pub set_main_window_loc_x: unsafe extern "system" fn(WORD, USHORT) -> USHORT,
    pub get_main_window_loc_y: unsafe extern "system" fn(WORD) -> USHORT,
    pub set_main_window_loc_y: unsafe extern "system" fn(WORD, USHORT) -> USHORT,
    pub get_main_window_width: unsafe extern "system" fn(WORD) -> USHORT,
    pub set_main_window_width: unsafe extern "system" fn(WORD, USHORT) -> USHORT,
    pub get_main_window_height: unsafe extern "system" fn(WORD) -> USHORT,
    pub set_main_window_height: unsafe extern "system" fn(WORD, USHORT) -> USHORT,
    pub get_fixed_emissivity: unsafe extern "system" fn(WORD) -> f32,
    pub set_fixed_emissivity: unsafe extern "system" fn(WORD, f32) -> f32,
    pub get_fixed_transmissivity: unsafe extern "system" fn(WORD) -> f32,
    pub set_fixed_transmissivity: unsafe extern "system" fn(WORD, f32) -> f32,
    pub get_fixed_temp_ambient: unsafe extern "system" fn(WORD) -> f32,
    pub set_fixed_temp_ambient: unsafe extern "system" fn(WORD, f32) -> f32,
    pub set_pif_out: unsafe extern "system" fn(WORD, WORD, f32) -> f32,
    pub fail_safe: unsafe extern "system" fn(WORD, BOOL) -> BOOL,
    
    // Hardware information
    pub get_hardware_model: unsafe extern "system" fn(WORD) -> UCHAR,
    pub get_hardware_spec: unsafe extern "system" fn(WORD) -> UCHAR,
    pub get_serial_number: unsafe extern "system" fn(WORD) -> ULONG,
    pub get_serial_number_ulis: unsafe extern "system" fn(WORD) -> ULONG,
    pub get_avg_time_per_frame: unsafe extern "system" fn(WORD) -> ULONG,
    pub get_visible_avg_time_per_frame: unsafe extern "system" fn(WORD) -> ULONG,
    pub get_firmware_msp: unsafe extern "system" fn(WORD) -> USHORT,
    pub get_firmware_cypress: unsafe extern "system" fn(WORD) -> USHORT,
    pub get_hardware_rev: unsafe extern "system" fn(WORD) -> USHORT,
    pub get_firmware_rev: unsafe extern "system" fn(WORD) -> USHORT,
    pub get_pid: unsafe extern "system" fn(WORD) -> USHORT,
    pub get_vid: unsafe extern "system" fn(WORD) -> USHORT,
    pub get_pif_serial_number: unsafe extern "system" fn(WORD) -> ULONG,
    pub get_pif_version: unsafe extern "system" fn(WORD) -> USHORT,
    
    // Control commands
    pub reset_flag: unsafe extern "system" fn(WORD),
    pub renew_flag: unsafe extern "system" fn(WORD) -> BOOL,
    pub close_application: unsafe extern "system" fn(WORD),
    pub reinit_device: unsafe extern "system" fn(WORD),
    pub file_snapshot: unsafe extern "system" fn(WORD),
    pub file_screenshot: unsafe extern "system" fn(WORD),
    pub file_record: unsafe extern "system" fn(WORD),
    pub file_stop: unsafe extern "system" fn(WORD),
    pub file_play: unsafe extern "system" fn(WORD),
    pub file_pause: unsafe extern "system" fn(WORD),
    pub file_open: unsafe extern "system" fn(WORD, *mut u16) -> USHORT,
    pub load_layout: unsafe extern "system" fn(WORD, *mut u16) -> USHORT,
    pub load_current_layout: unsafe extern "system" fn(WORD),
    pub save_current_layout: unsafe extern "system" fn(WORD),
    pub set_standard_layout: unsafe extern "system" fn(WORD),
    pub master_instance_name: unsafe extern "system" fn(WORD, *mut u16) -> USHORT,
}

// Wrapper struct for safe access to the library
pub struct ImagerIPC2Library {
    pub bindings: ImagerIPC2Bindings,
}

impl ImagerIPC2Library {
    pub fn new() -> Result<Self> {
        let module = Self::load_library()?;
        let bindings = unsafe { Self::get_bindings(module)? };
        
        Ok(Self { bindings })
    }

    fn load_library() -> Result<HMODULE> {
        // Determine architecture and version to load appropriate DLL
        let arch = if cfg!(target_arch = "x86") { "x86" } else { "x64" };
        let version = "v120"; // Default to latest version
        
        // Try to find the appropriate DLL file
        let dll_path = format!("Connect_SDK/Lib/{}/ImagerIPC2{}.dll", version, 
                              if arch == "x64" { "x64" } else { "" });
        
        debug!("Attempting to load DLL: {}", dll_path);
        
        let wide_path: Vec<u16> = OsString::from(&dll_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        
        let module = unsafe {
            LoadLibraryW(PCWSTR(wide_path.as_ptr()))
        };
        
        if module.is_err() {
            error!("Failed to load ImagerIPC2 DLL: {}", dll_path);
            return Err(anyhow::anyhow!("Failed to load ImagerIPC2 DLL"));
        }
        
        info!("Successfully loaded ImagerIPC2 DLL: {}", dll_path);
        Ok(module.unwrap().into())
    }

    unsafe fn get_bindings(module: HMODULE) -> Result<ImagerIPC2Bindings> {
        macro_rules! get_func {
            ($name:expr, $type:ty) => {{
                let func_name = concat!($name, "\0").as_bytes().as_ptr();
                let func_ptr = GetProcAddress(module, PCSTR(func_name as *const u8));
                if func_ptr.is_none() {
                    error!("Failed to get function pointer for: {}", $name);
                    return Err(anyhow::anyhow!("Failed to get function pointer for: {}", $name));
                }
                std::mem::transmute(func_ptr)
            }};
        }

        Ok(ImagerIPC2Bindings {
            // Initialization and control functions
            set_imager_ipc_count: get_func!("SetImagerIPCCount", unsafe extern "system" fn(WORD) -> HRESULT),
            init_imager_ipc: get_func!("InitImagerIPC", unsafe extern "system" fn(WORD) -> HRESULT),
            init_named_imager_ipc: get_func!("InitNamedImagerIPC", unsafe extern "system" fn(WORD, *mut u16) -> HRESULT),
            run_imager_ipc: get_func!("RunImagerIPC", unsafe extern "system" fn(WORD) -> HRESULT),
            start_imager_ipc: get_func!("StartImagerIPC", unsafe extern "system" fn(WORD) -> HRESULT),
            release_imager_ipc: get_func!("ReleaseImagerIPC", unsafe extern "system" fn(WORD) -> HRESULT),
            imager_ipc_process_messages: get_func!("ImagerIPCProcessMessages", unsafe extern "system" fn(WORD) -> HRESULT),
            acknowledge_frame: get_func!("AcknowledgeFrame", unsafe extern "system" fn(WORD) -> HRESULT),
            
            // Frame functions
            get_frame_config: get_func!("GetFrameConfig", unsafe extern "system" fn(WORD, *mut i32, *mut i32, *mut i32) -> HRESULT),
            get_visible_frame_config: get_func!("GetVisibleFrameConfig", unsafe extern "system" fn(WORD, *mut i32, *mut i32, *mut i32) -> HRESULT),
            get_frame: get_func!("GetFrame", unsafe extern "system" fn(WORD, WORD, *mut std::ffi::c_void, u32, *mut FrameMetadata) -> HRESULT),
            get_visible_frame: get_func!("GetVisibleFrame", unsafe extern "system" fn(WORD, WORD, *mut std::ffi::c_void, u32, *mut FrameMetadata) -> HRESULT),
            
            // Logging functions
            set_log_file: get_func!("SetLogFile", unsafe extern "system" fn(*mut u16, i32, BOOL) -> HRESULT),
            set_logging: get_func!("SetLogging", unsafe extern "system" fn(i32) -> HRESULT),
            log: get_func!("Log", unsafe extern "system" fn(WORD, *mut i8, i32) -> HRESULT),
            
            // Callback functions
            set_callback_on_server_stopped: get_func!("SetCallback_OnServerStopped", unsafe extern "system" fn(WORD, FpOnServerStopped) -> HRESULT),
            set_callback_on_frame_init: get_func!("SetCallback_OnFrameInit", unsafe extern "system" fn(WORD, FpOnFrameInit) -> HRESULT),
            set_callback_on_new_frame: get_func!("SetCallback_OnNewFrame", unsafe extern "system" fn(WORD, FpOnNewFrame) -> HRESULT),
            set_callback_on_new_frame_ex: get_func!("SetCallback_OnNewFrameEx", unsafe extern "system" fn(WORD, FpOnNewFrameEx) -> HRESULT),
            set_callback_on_visible_frame_init: get_func!("SetCallback_OnVisibleFrameInit", unsafe extern "system" fn(WORD, FpOnFrameInit) -> HRESULT),
            set_callback_on_new_visible_frame: get_func!("SetCallback_OnNewVisibleFrame", unsafe extern "system" fn(WORD, FpOnNewFrame) -> HRESULT),
            set_callback_on_new_visible_frame_ex: get_func!("SetCallback_OnNewVisibleFrameEx", unsafe extern "system" fn(WORD, FpOnNewFrameEx) -> HRESULT),
            set_callback_on_init_completed: get_func!("SetCallback_OnInitCompleted", unsafe extern "system" fn(WORD, FpOnInitCompleted) -> HRESULT),
            set_callback_on_config_changed: get_func!("SetCallback_OnConfigChanged", unsafe extern "system" fn(WORD, FpOnConfigChanged) -> HRESULT),
            set_callback_on_file_command_ready: get_func!("SetCallback_OnFileCommandReady", unsafe extern "system" fn(WORD, FpOnStringSend) -> HRESULT),
            set_callback_on_new_nmea_string: get_func!("SetCallback_OnNewNMEAString", unsafe extern "system" fn(WORD, FpOnStringSend) -> HRESULT),
            
            // Get/Set procedures
            get_version_application: get_func!("GetVersionApplication", unsafe extern "system" fn(WORD) -> i64),
            get_version_hid_dll: get_func!("GetVersionHID_DLL", unsafe extern "system" fn(WORD) -> i64),
            get_version_cd_dll: get_func!("GetVersionCD_DLL", unsafe extern "system" fn(WORD) -> i64),
            get_version_ipc_dll: get_func!("GetVersionIPC_DLL", unsafe extern "system" fn(WORD) -> i64),
            
            get_temp_chip: get_func!("GetTempChip", unsafe extern "system" fn(WORD) -> f32),
            get_temp_flag: get_func!("GetTempFlag", unsafe extern "system" fn(WORD) -> f32),
            get_temp_proc: get_func!("GetTempProc", unsafe extern "system" fn(WORD) -> f32),
            get_temp_box: get_func!("GetTempBox", unsafe extern "system" fn(WORD) -> f32),
            get_temp_housing: get_func!("GetTempHousing", unsafe extern "system" fn(WORD) -> f32),
            get_temp_target: get_func!("GetTempTarget", unsafe extern "system" fn(WORD) -> f32),
            get_humidity: get_func!("GetHumidity", unsafe extern "system" fn(WORD) -> f32),
            get_temp_range_count: get_func!("GetTempRangeCount", unsafe extern "system" fn(WORD) -> USHORT),
            get_optics_count: get_func!("GetOpticsCount", unsafe extern "system" fn(WORD) -> USHORT),
            get_measure_area_count: get_func!("GetMeasureAreaCount", unsafe extern "system" fn(WORD) -> USHORT),
            get_video_format_count: get_func!("GetVideoFormatCount", unsafe extern "system" fn(WORD) -> USHORT),
            get_clipped_format_max_pos: get_func!("GetClippedFormatMaxPos", unsafe extern "system" fn(WORD, *mut POINT) -> HRESULT),
            get_temp_min_range: get_func!("GetTempMinRange", unsafe extern "system" fn(WORD, ULONG) -> f32),
            get_temp_max_range: get_func!("GetTempMaxRange", unsafe extern "system" fn(WORD, ULONG) -> f32),
            get_optics_fov: get_func!("GetOpticsFOV", unsafe extern "system" fn(WORD, ULONG) -> USHORT),
            get_optics_serial_number: get_func!("GetOpticsSerialNumber", unsafe extern "system" fn(WORD, ULONG) -> ULONG),
            get_temp_measure_area: get_func!("GetTempMeasureArea", unsafe extern "system" fn(WORD, ULONG) -> f32),
            get_loc_measure_area: get_func!("GetLocMeasureArea", unsafe extern "system" fn(WORD, ULONG, *mut POINT) -> HRESULT),
            set_loc_measure_area: get_func!("SetLocMeasureArea", unsafe extern "system" fn(WORD, ULONG, POINT) -> HRESULT),
            get_init_counter: get_func!("GetInitCounter", unsafe extern "system" fn(WORD) -> USHORT),
            get_ipc_state: get_func!("GetIPCState", unsafe extern "system" fn(WORD, BOOL) -> USHORT),
            get_ipc_mode: get_func!("GetIPCMode", unsafe extern "system" fn(WORD) -> USHORT),
            set_ipc_mode: get_func!("SetIPCMode", unsafe extern "system" fn(WORD, USHORT) -> USHORT),
            get_frame_queue: get_func!("GetFrameQueue", unsafe extern "system" fn(WORD) -> USHORT),
            get_visible_frame_queue: get_func!("GetVisibleFrameQueue", unsafe extern "system" fn(WORD) -> USHORT),
            get_video_format: get_func!("GetVideoFormat", unsafe extern "system" fn(WORD, ULONG, *mut VideoFormat) -> HRESULT),
            get_ir_arranging: get_func!("GetIRArranging", unsafe extern "system" fn(WORD, *mut IRArranging) -> HRESULT),
            set_ir_arranging: get_func!("SetIRArranging", unsafe extern "system" fn(WORD, *mut IRArranging) -> HRESULT),
            get_path_of_stored_file: get_func!("GetPathOfStoredFile", unsafe extern "system" fn(WORD, *mut u16, i32) -> HRESULT),
            get_new_nmea_string: get_func!("GetNewNMEAString", unsafe extern "system" fn(WORD, *mut u16, i32) -> HRESULT),
            get_measure_area: get_func!("GetMeasureArea", unsafe extern "system" fn(WORD, ULONG, *mut MeasureArea) -> HRESULT),
            set_measure_area: get_func!("SetMeasureArea", unsafe extern "system" fn(WORD, ULONG, *mut MeasureArea, BOOL) -> HRESULT),
            remove_measure_area: get_func!("RemoveMeasureArea", unsafe extern "system" fn(WORD, ULONG) -> HRESULT),
            add_measure_area_point: get_func!("AddMeasureAreaPoint", unsafe extern "system" fn(WORD, ULONG, POINT) -> HRESULT),
            set_measure_area_name: get_func!("SetMeasureAreaName", unsafe extern "system" fn(WORD, ULONG, *mut u16) -> HRESULT),
            get_measure_area_name: get_func!("GetMeasureAreaName", unsafe extern "system" fn(WORD, ULONG, *mut u16, *mut i32, i32) -> HRESULT),
            
            // Flag and device control
            get_flag: get_func!("GetFlag", unsafe extern "system" fn(WORD) -> BOOL),
            set_flag: get_func!("SetFlag", unsafe extern "system" fn(WORD, BOOL) -> BOOL),
            get_optics_index: get_func!("GetOpticsIndex", unsafe extern "system" fn(WORD) -> USHORT),
            set_optics_index: get_func!("SetOpticsIndex", unsafe extern "system" fn(WORD, USHORT) -> USHORT),
            get_temp_range_index: get_func!("GetTempRangeIndex", unsafe extern "system" fn(WORD) -> USHORT),
            set_temp_range_index: get_func!("SetTempRangeIndex", unsafe extern "system" fn(WORD, USHORT) -> USHORT),
            get_video_format_index: get_func!("GetVideoFormatIndex", unsafe extern "system" fn(WORD) -> USHORT),
            set_video_format_index: get_func!("SetVideoFormatIndex", unsafe extern "system" fn(WORD, USHORT) -> USHORT),
            get_clipped_format_pos: get_func!("GetClippedFormatPos", unsafe extern "system" fn(WORD, *mut POINT) -> HRESULT),
            set_clipped_format_pos: get_func!("SetClippedFormatPos", unsafe extern "system" fn(WORD, POINT) -> HRESULT),
            get_temp_range_decimal: get_func!("GetTempRangeDecimal", unsafe extern "system" fn(WORD, BOOL) -> USHORT),
            get_main_window_embedded: get_func!("GetMainWindowEmbedded", unsafe extern "system" fn(WORD) -> BOOL),
            set_main_window_embedded: get_func!("SetMainWindowEmbedded", unsafe extern "system" fn(WORD, BOOL) -> BOOL),
            get_main_window_loc_x: get_func!("GetMainWindowLocX", unsafe extern "system" fn(WORD) -> USHORT),
            set_main_window_loc_x: get_func!("SetMainWindowLocX", unsafe extern "system" fn(WORD, USHORT) -> USHORT),
            get_main_window_loc_y: get_func!("GetMainWindowLocY", unsafe extern "system" fn(WORD) -> USHORT),
            set_main_window_loc_y: get_func!("SetMainWindowLocY", unsafe extern "system" fn(WORD, USHORT) -> USHORT),
            get_main_window_width: get_func!("GetMainWindowWidth", unsafe extern "system" fn(WORD) -> USHORT),
            set_main_window_width: get_func!("SetMainWindowWidth", unsafe extern "system" fn(WORD, USHORT) -> USHORT),
            get_main_window_height: get_func!("GetMainWindowHeight", unsafe extern "system" fn(WORD) -> USHORT),
            set_main_window_height: get_func!("SetMainWindowHeight", unsafe extern "system" fn(WORD, USHORT) -> USHORT),
            get_fixed_emissivity: get_func!("GetFixedEmissivity", unsafe extern "system" fn(WORD) -> f32),
            set_fixed_emissivity: get_func!("SetFixedEmissivity", unsafe extern "system" fn(WORD, f32) -> f32),
            get_fixed_transmissivity: get_func!("GetFixedTransmissivity", unsafe extern "system" fn(WORD) -> f32),
            set_fixed_transmissivity: get_func!("SetFixedTransmissivity", unsafe extern "system" fn(WORD, f32) -> f32),
            get_fixed_temp_ambient: get_func!("GetFixedTempAmbient", unsafe extern "system" fn(WORD) -> f32),
            set_fixed_temp_ambient: get_func!("SetFixedTempAmbient", unsafe extern "system" fn(WORD, f32) -> f32),
            set_pif_out: get_func!("SetPifOut", unsafe extern "system" fn(WORD, WORD, f32) -> f32),
            fail_safe: get_func!("FailSafe", unsafe extern "system" fn(WORD, BOOL) -> BOOL),
            
            // Hardware information
            get_hardware_model: get_func!("GetHardware_Model", unsafe extern "system" fn(WORD) -> UCHAR),
            get_hardware_spec: get_func!("GetHardware_Spec", unsafe extern "system" fn(WORD) -> UCHAR),
            get_serial_number: get_func!("GetSerialNumber", unsafe extern "system" fn(WORD) -> ULONG),
            get_serial_number_ulis: get_func!("GetSerialNumberULIS", unsafe extern "system" fn(WORD) -> ULONG),
            get_avg_time_per_frame: get_func!("GetAvgTimePerFrame", unsafe extern "system" fn(WORD) -> ULONG),
            get_visible_avg_time_per_frame: get_func!("GetVisibleAvgTimePerFrame", unsafe extern "system" fn(WORD) -> ULONG),
            get_firmware_msp: get_func!("GetFirmware_MSP", unsafe extern "system" fn(WORD) -> USHORT),
            get_firmware_cypress: get_func!("GetFirmware_Cypress", unsafe extern "system" fn(WORD) -> USHORT),
            get_hardware_rev: get_func!("GetHardwareRev", unsafe extern "system" fn(WORD) -> USHORT),
            get_firmware_rev: get_func!("GetFirmwareRev", unsafe extern "system" fn(WORD) -> USHORT),
            get_pid: get_func!("GetPID", unsafe extern "system" fn(WORD) -> USHORT),
            get_vid: get_func!("GetVID", unsafe extern "system" fn(WORD) -> USHORT),
            get_pif_serial_number: get_func!("GetPIFSerialNumber", unsafe extern "system" fn(WORD) -> ULONG),
            get_pif_version: get_func!("GetPIFVersion", unsafe extern "system" fn(WORD) -> USHORT),
            
            // Control commands
            reset_flag: get_func!("ResetFlag", unsafe extern "system" fn(WORD)),
            renew_flag: get_func!("RenewFlag", unsafe extern "system" fn(WORD) -> BOOL),
            close_application: get_func!("CloseApplication", unsafe extern "system" fn(WORD)),
            reinit_device: get_func!("ReinitDevice", unsafe extern "system" fn(WORD)),
            file_snapshot: get_func!("FileSnapshot", unsafe extern "system" fn(WORD)),
            file_screenshot: get_func!("FileScreenshot", unsafe extern "system" fn(WORD)),
            file_record: get_func!("FileRecord", unsafe extern "system" fn(WORD)),
            file_stop: get_func!("FileStop", unsafe extern "system" fn(WORD)),
            file_play: get_func!("FilePlay", unsafe extern "system" fn(WORD)),
            file_pause: get_func!("FilePause", unsafe extern "system" fn(WORD)),
            file_open: get_func!("FileOpen", unsafe extern "system" fn(WORD, *mut u16) -> USHORT),
            load_layout: get_func!("LoadLayout", unsafe extern "system" fn(WORD, *mut u16) -> USHORT),
            load_current_layout: get_func!("LoadCurrentLayout", unsafe extern "system" fn(WORD)),
            save_current_layout: get_func!("SaveCurrentLayout", unsafe extern "system" fn(WORD)),
            set_standard_layout: get_func!("SetStandardLayout", unsafe extern "system" fn(WORD)),
            master_instance_name: get_func!("MasterInstanceName", unsafe extern "system" fn(WORD, *mut u16) -> USHORT),
        })
    }
}

// Safe wrapper for the library functions
pub struct ImagerIPC2 {
    pub library: ImagerIPC2Library,
}

impl ImagerIPC2 {
    pub fn new() -> Result<Self> {
        let library = ImagerIPC2Library::new()?;
        Ok(Self { library })
    }

    // Safe wrapper functions with parameter validation
   pub fn init_imager_ipc(&self, index: u16) -> Result<()> {
       if index >= 100 {  // Reasonable upper limit
           return Err(anyhow::anyhow!("Invalid camera index: {}", index));
       }
       let result = unsafe { (self.library.bindings.init_imager_ipc)(index) };
       Self::check_hresult(result)?;
       Ok(())
   }

   pub fn run_imager_ipc(&self, index: u16) -> Result<()> {
       if index >= 100 {
           return Err(anyhow::anyhow!("Invalid camera index: {}", index));
       }
       let result = unsafe { (self.library.bindings.run_imager_ipc)(index) };
       Self::check_hresult(result)?;
       Ok(())
   }

   pub fn release_imager_ipc(&self, index: u16) -> Result<()> {
       if index >= 100 {
           return Err(anyhow::anyhow!("Invalid camera index: {}", index));
       }
       let result = unsafe { (self.library.bindings.release_imager_ipc)(index) };
       Self::check_hresult(result)?;
       Ok(())
   }

    pub fn get_frame_config(&self, index: u16) -> Result<(i32, i32, i32)> {
        if index >= 100 {
            return Err(anyhow::anyhow!("Invalid camera index: {}", index));
        }
        
        let mut width = 0i32;
        let mut height = 0i32;
        let mut depth = 0i32;
        
        let result = unsafe {
            (self.library.bindings.get_frame_config)(index, &mut width, &mut height, &mut depth)
        };
        Self::check_hresult(result)?;
        
        Ok((width, height, depth))
    }

    pub fn get_frame(&self, index: u16, timeout: u16, buffer: &mut [u8], metadata: &mut FrameMetadata) -> Result<()> {
        if index >= 100 {
            return Err(anyhow::anyhow!("Invalid camera index: {}", index));
        }
        
        let result = unsafe { 
            (self.library.bindings.get_frame)(
                index, 
                timeout, 
                buffer.as_mut_ptr() as *mut std::ffi::c_void, 
                buffer.len() as u32, 
                metadata
            ) 
        };
        Self::check_hresult(result)?;
        Ok(())
    }


    pub fn get_temp_chip(&self, index: u16) -> Result<f32> {
        if index >= 100 {
            return Err(anyhow::anyhow!("Invalid camera index: {}", index));
        }
        let temp = unsafe { (self.library.bindings.get_temp_chip)(index) };
        Ok(temp)
    }

    pub fn get_temp_flag(&self, index: u16) -> Result<f32> {
        if index >= 100 {
            return Err(anyhow::anyhow!("Invalid camera index: {}", index));
        }
        let temp = unsafe { (self.library.bindings.get_temp_flag)(index) };
        Ok(temp)
    }


    pub fn close_application(&self, index: u16) -> Result<()> {
        if index >= 100 {
            return Err(anyhow::anyhow!("Invalid camera index: {}", index));
        }
        unsafe { (self.library.bindings.close_application)(index) };
        Ok(())
    }

    pub fn file_snapshot(&self, index: u16) -> Result<()> {
        if index >= 100 {
            return Err(anyhow::anyhow!("Invalid camera index: {}", index));
        }
        unsafe { (self.library.bindings.file_snapshot)(index) };
        Ok(())
    }

    // Helper function to check HRESULT values
    pub fn check_hresult(result: i32) -> Result<()> {
        if result < 0 {
            Err(anyhow::anyhow!("HRESULT error: 0x{:X}", result as u32))
        } else {
            Ok(())
        }
    }
}
