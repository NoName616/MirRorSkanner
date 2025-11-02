use anyhow::Result;
use std::sync::{Arc, Mutex, RwLock, LazyLock};
use std::collections::HashMap;

use crate::ffi::{ImagerIPC2, FrameMetadata, TFlagState, HRESULT};

// Global registry for camera instances to be accessible from callbacks
static CAMERA_INSTANCES: LazyLock<RwLock<HashMap<u16, Arc<Mutex<CameraState>>>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

// Define the CameraMode enum used in the app
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CameraMode {
    Callback,
    Polling,
}

// Define the ImageType enum used in the app
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraImage {
    pub data: Vec<f32>,
    pub width: u32,
    pub height: u32,
    #[serde(
        serialize_with = "serialize_system_time",
        deserialize_with = "deserialize_system_time"
    )]
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraImageU8 {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    #[serde(
        serialize_with = "serialize_system_time",
        deserialize_with = "deserialize_system_time"
    )]
    pub timestamp: std::time::SystemTime,
}

// Serialization helper for SystemTime
fn serialize_system_time<S>(time: &std::time::SystemTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::Serialize;
    let duration = time.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap();
    duration.as_secs().serialize(serializer)
}

// Deserialization helper for SystemTime
fn deserialize_system_time<'de, D>(deserializer: D) -> Result<std::time::SystemTime, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let secs = u64::deserialize(deserializer)?;
    Ok(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs))
}

// Define the TemperatureData struct used in the app
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TemperatureData {
    pub chip_temp: f32,
    pub flag_temp: f32,
    pub housing_temp: f32,
}

// Define the FlagState enum used in the app
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, Default)]
pub enum FlagState {
    Open,
    Closed,
    #[default]
    Unknown,
}

// Define the CameraController struct that wraps the Camera functionality for the UI
pub struct CameraController {
    camera: Camera,
    mode: CameraMode,
    running: bool,
}

impl CameraController {
    pub fn new(_library_version: String) -> Result<Self> {
        // The library version is used to load the appropriate DLL, but for this implementation
        // we'll use the existing Camera initialization which handles this internally
        let camera = Camera::new(0)?; // Use index 0 by default
        Ok(Self {
            camera,
            mode: CameraMode::Callback,
            running: false,
        })
    }

    pub fn connect(&mut self) -> Result<()> {
        match self.mode {
            CameraMode::Callback => self.camera.connect_callback(),
            CameraMode::Polling => self.camera.connect_polling(),
        }?;
        self.running = true;
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<()> {
        self.camera.disconnect()?;
        self.running = false;
        Ok(())
    }

    pub fn set_mode(&mut self, mode: CameraMode) -> Result<()> {
        if self.running {
            // Need to disconnect first before changing mode
            self.disconnect()?;
            self.mode = mode;
            self.connect()?;
        } else {
            self.mode = mode;
        }
        Ok(())
    }

    pub fn get_camera_state(&self) -> Result<CameraStateData> {
        let mut state = CameraStateData::default();
        
        if self.camera.is_connected() {
            state.temperature_data.chip_temp = self.camera.get_temp_chip().unwrap_or(0.0);
            state.temperature_data.flag_temp = self.camera.get_temp_flag().unwrap_or(0.0);
            state.temperature_data.housing_temp = self.camera.get_temp_housing().unwrap_or(0.0);
            
            state.flag_state = if self.camera.get_flag().unwrap_or(false) {
                FlagState::Open
            } else {
                FlagState::Closed
            };

            if let Ok((frame, metadata)) = self.camera.get_frame(0) {
                state.ir_image = Some(CameraImage {
                    data: self.camera.extract_temperatures_from_frame(&frame).unwrap_or_default(),
                    width: metadata.pi_fin[0] as u32,
                    height: metadata.pi_fin[1] as u32,
                    timestamp: std::time::SystemTime::now(),
                });
                state.frame_count = metadata.counter as u64;
            }

            if let Ok((frame, metadata)) = self.camera.get_visible_frame(0) {
                // For visible image, we just pass the raw bytes.
                // We need a separate field for this in CameraStateData or a different image type.
                // For now, let's assume visible_image in CameraStateData will store raw bytes as f32 for simplicity, which is wrong.
                // This part needs a proper refactor.
                state.visible_image = Some(CameraImageU8 {
                    data: frame,
                    width: metadata.pi_fin[0] as u32,
                    height: metadata.pi_fin[1] as u32,
                    timestamp: std::time::SystemTime::now(),
                });
            }
        }
        
        Ok(state)
    }

    pub fn start_recording(&self) -> Result<()> {
        if self.camera.is_connected() {
            self.camera.file_record()
        } else {
            Err(anyhow::anyhow!("Camera not connected"))
        }
    }

    pub fn stop_recording(&self) -> Result<()> {
        if self.camera.is_connected() {
            self.camera.file_stop()
        } else {
            Err(anyhow::anyhow!("Camera not connected"))
        }
    }

    pub fn capture_snapshot(&self) -> Result<()> {
        if self.camera.is_connected() {
            self.camera.file_snapshot()
        } else {
            Err(anyhow::anyhow!("Camera not connected"))
        }
    }

    pub fn capture_screenshot(&self) -> Result<()> {
        if self.camera.is_connected() {
            self.camera.file_screenshot()
        } else {
            Err(anyhow::anyhow!("Camera not connected"))
        }
    }
}

// Define the CameraStateData struct that the UI uses
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CameraStateData {
    pub temperature_data: TemperatureData,
    pub flag_state: FlagState,
    pub frame_count: u64,
    pub ir_image: Option<CameraImage>,
    pub visible_image: Option<CameraImageU8>,
}

/// Camera parameters structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CameraParameters {
    pub temperature_range_min: f32,
    pub temperature_range_max: f32,
    pub emissivity: f32,
    pub distance: f32,
}

impl Default for CameraParameters {
    fn default() -> Self {
        Self {
            temperature_range_min: 0.0,
            temperature_range_max: 100.0,
            emissivity: 0.95,
            distance: 1.0,
        }
    }
}

/// Represents the Optris Pi camera with full functionality
pub struct Camera {
    ipc: Arc<ImagerIPC2>,
    index: u16,
    connected: bool,
    frame_initialized: bool,
    callback_mode: bool,
    running: bool,
    // Shared state for callbacks
    _state: Arc<Mutex<CameraState>>,
}

#[derive(Debug, Clone)]
struct CameraState {
    index: u16,
    connected: bool,
    frame_initialized: bool,
    stopped: bool,
    last_metadata: Option<FrameMetadata>,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            index: 0,
            connected: false,
            frame_initialized: false,
            stopped: false,
            last_metadata: None,
        }
    }
}

impl Camera {
    /// Creates a new instance of the Camera
    pub fn new(index: u16) -> Result<Self> {
        let ipc = Arc::new(ImagerIPC2::new()?);
        let mut state = CameraState::default();
        state.index = index;
        let state = Arc::new(Mutex::new(state));
        
        // Register this camera instance globally so callbacks can access it
        {
            let mut instances = CAMERA_INSTANCES.write().unwrap();
            instances.insert(index, Arc::new(Mutex::new(CameraState::default())));
        }
        
        Ok(Self {
            ipc,
            index,
            connected: false,
            frame_initialized: false,
            callback_mode: false,
            running: false,
            _state: state,
        })
    }

    /// Connects to the camera using the callback mode
    pub fn connect_callback(&mut self) -> Result<()> {
        // Update the global state with the correct index
        {
            let mut instances = CAMERA_INSTANCES.write().unwrap();
            let mut state = CameraState::default();
            state.index = self.index;
            instances.insert(self.index, Arc::new(Mutex::new(state)));
        }
        
        self.ipc.init_imager_ipc(self.index)?;
        self.setup_callbacks()?;
        self.ipc.run_imager_ipc(self.index)?;
        self.connected = true;
        self.callback_mode = true;
        self.running = true;
        Ok(())
    }

    /// Connects to the camera using the polling mode
    pub fn connect_polling(&mut self) -> Result<()> {
        self.ipc.init_imager_ipc(self.index)?;
        self.ipc.run_imager_ipc(self.index)?;
        self.connected = true;
        self.callback_mode = false;
        self.running = true;
        Ok(())
    }

    /// Disconnects from the camera
    pub fn disconnect(&mut self) -> Result<()> {
        if self.running {
            self.running = false;
            self.ipc.close_application(self.index)?;
            self.ipc.release_imager_ipc(self.index)?;
        }
        self.connected = false;
        self.frame_initialized = false;
        Ok(())
    }

    /// Checks if the camera is connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Gets camera frame configuration
    pub fn get_frame_config(&self) -> Result<(i32, i32, i32)> {
        if !self.connected {
            return Err(anyhow::anyhow!("Camera not connected"));
        }
        self.ipc.get_frame_config(self.index).map(|(w, h, d)| (w, h, d))
    }

    /// Gets visible frame configuration
    pub fn get_visible_frame_config(&self) -> Result<(i32, i32, i32)> {
        if !self.connected {
            return Err(anyhow::anyhow!("Camera not connected"));
        }
        let mut width = 0i32;
        let mut height = 0i32;
        let mut depth = 0i32;
        let result = unsafe {
            (self.ipc.library.bindings.get_visible_frame_config)(
                self.index,
                &mut width,
                &mut height,
                &mut depth
            )
        };
        crate::ffi::ImagerIPC2::check_hresult(result)?;
        Ok((width, height, depth))
    }

    /// Gets a frame from the camera with timeout
    pub fn get_frame(&self, timeout: u16) -> Result<(Vec<u8>, FrameMetadata)> {
        if !self.connected || !self.frame_initialized {
            return Err(anyhow::anyhow!("Camera not connected or frame not initialized"));
        }

        let (width, height, depth) = self.get_frame_config()?;
        let buffer_size = (width * height * depth / 8) as usize;
        let mut buffer = vec![0u8; buffer_size];
        let mut metadata = FrameMetadata {
            size: std::mem::size_of::<FrameMetadata>() as u16,
            counter: 0,
            counter_hw: 0,
            timestamp: 0,
            timestamp_media: 0,
            flag_state: TFlagState::FsFlagClose,
            temp_chip: 0.0,
            temp_flag: 0.0,
            temp_box: 0.0,
            pi_fin: [0, 0],
        };

        self.ipc.get_frame(self.index, timeout, &mut buffer, &mut metadata)?;
        Ok((buffer, metadata))
    }

    /// Gets a visible frame from the camera with timeout
    pub fn get_visible_frame(&self, timeout: u16) -> Result<(Vec<u8>, FrameMetadata)> {
        if !self.connected || !self.frame_initialized {
            return Err(anyhow::anyhow!("Camera not connected or frame not initialized"));
        }

        let (width, height, depth) = self.get_visible_frame_config()?;
        let buffer_size = (width * height * depth / 8) as usize;
        let mut buffer = vec![0u8; buffer_size];
        let mut metadata = FrameMetadata {
            size: std::mem::size_of::<FrameMetadata>() as u16,
            counter: 0,
            counter_hw: 0,
            timestamp: 0,
            timestamp_media: 0,
            flag_state: TFlagState::FsFlagClose,
            temp_chip: 0.0,
            temp_flag: 0.0,
            temp_box: 0.0,
            pi_fin: [0, 0],
        };

        let result = unsafe {
            (self.ipc.library.bindings.get_visible_frame)(
                self.index,
                timeout,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                buffer.len() as u32,
                &mut metadata
            )
        };
        crate::ffi::ImagerIPC2::check_hresult(result)?;
        Ok((buffer, metadata))
    }


    /// Processes frame data to extract temperature values
    pub fn extract_temperatures_from_frame(&self, frame_data: &[u8]) -> Result<Vec<f32>> {
        if !self.connected || !self.frame_initialized {
            return Err(anyhow::anyhow!("Camera not connected or frame not initialized"));
        }

        // Convert raw frame data to temperature values
        // This is a simplified implementation - in real usage, you'd need to use the camera's specific calibration
        let mut temperatures = Vec::new();
        for chunk in frame_data.chunks(2) { // Assuming 16-bit values
            if chunk.len() == 2 {
                let value = u16::from_le_bytes([chunk[0], chunk[1]]) as f32;
                // Apply some basic conversion - this would need to be calibrated based on the specific camera model
                temperatures.push(value / 10.0); // Placeholder conversion
            }
        }
        Ok(temperatures)
    }
}

/// Additional structures for frame information and statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrameInfo {
    pub frame_counter: u32,
    pub hardware_counter: u32,
    pub timestamp: i64,
    pub flag_state: TFlagState,
    pub chip_temperature: f32,
    pub flag_temperature: f32,
    pub box_temperature: f32,
    pub digital_input: bool,
    pub analog_input_1: u16,
    pub analog_input_2: u16,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrameStatistics {
    pub queue_size: u16,
    pub chip_temperature: f32,
    pub flag_temperature: f32,
    pub box_temperature: f32,
}
// This extra closing brace has been removed

impl Camera {

    /// Gets flag state
    pub fn get_flag(&self) -> Result<bool> {
        if !self.connected {
            return Err(anyhow::anyhow!("Camera not connected"));
        }
        let flag = unsafe { (self.ipc.library.bindings.get_flag)(self.index) };
        Ok(flag.as_bool())
    }


    /// Gets chip temperature
    pub fn get_temp_chip(&self) -> Result<f32> {
        if !self.connected {
            return Err(anyhow::anyhow!("Camera not connected"));
        }
        self.ipc.get_temp_chip(self.index)
    }

    /// Gets flag temperature
    pub fn get_temp_flag(&self) -> Result<f32> {
        if !self.connected {
            return Err(anyhow::anyhow!("Camera not connected"));
        }
        self.ipc.get_temp_flag(self.index)
    }


    /// Gets housing temperature
    pub fn get_temp_housing(&self) -> Result<f32> {
        if !self.connected {
            return Err(anyhow::anyhow!("Camera not connected"));
        }
        let temp = unsafe { (self.ipc.library.bindings.get_temp_housing)(self.index) };
        Ok(temp)
    }


    /// Takes a snapshot and saves it to a file
    pub fn file_snapshot(&self) -> Result<()> {
        if !self.connected {
            return Err(anyhow::anyhow!("Camera not connected"));
        }
        self.ipc.file_snapshot(self.index)
    }

    /// Takes a screenshot
    pub fn file_screenshot(&self) -> Result<()> {
        if !self.connected {
            return Err(anyhow::anyhow!("Camera not connected"));
        }
        unsafe { (self.ipc.library.bindings.file_screenshot)(self.index) };
        Ok(())
    }

    /// Starts recording
    pub fn file_record(&self) -> Result<()> {
        if !self.connected {
            return Err(anyhow::anyhow!("Camera not connected"));
        }
        unsafe { (self.ipc.library.bindings.file_record)(self.index) };
        Ok(())
    }

    /// Stops recording
    pub fn file_stop(&self) -> Result<()> {
        if !self.connected {
            return Err(anyhow::anyhow!("Camera not connected"));
        }
        unsafe { (self.ipc.library.bindings.file_stop)(self.index) };
        Ok(())
    }



    /// Sets up callbacks for callback mode
    fn setup_callbacks(&mut self) -> Result<()> {
        unsafe {
            (self.ipc.library.bindings.set_callback_on_server_stopped)(self.index, Some(on_server_stopped_callback));
            (self.ipc.library.bindings.set_callback_on_frame_init)(self.index, Some(on_frame_init_callback));
            (self.ipc.library.bindings.set_callback_on_new_frame_ex)(self.index, Some(on_new_frame_ex_callback));
            (self.ipc.library.bindings.set_callback_on_init_completed)(self.index, Some(on_init_completed_callback));
        }
        Ok(())
    }
}

/// Callback for when server stops
extern "system" fn on_server_stopped_callback(_reason: i32) -> HRESULT {
    if let Ok(instances) = CAMERA_INSTANCES.read() {
        if let Some(camera_state) = instances.get(&0) {
            if let Ok(mut state) = camera_state.lock() {
                state.stopped = true;
            }
        }
    }
    0 // S_OK
}

/// Callback for when frame is initialized
extern "system" fn on_frame_init_callback(_width: i32, _height: i32, _depth: i32) -> HRESULT {
    if let Ok(instances) = CAMERA_INSTANCES.read() {
        if let Some(camera_state) = instances.get(&0) {
            if let Ok(mut state) = camera_state.lock() {
                state.frame_initialized = true;
            }
        }
    }
    0 // S_OK
}

/// Callback for when new frame is available with metadata
extern "system" fn on_new_frame_ex_callback(_buffer: *mut std::ffi::c_void, metadata: *mut FrameMetadata) -> HRESULT {
    if let Ok(instances) = CAMERA_INSTANCES.read() {
        if let Some(camera_state) = instances.get(&0) {
            if let Ok(mut state) = camera_state.lock() {
                state.last_metadata = Some(unsafe { *metadata });
            }
        }
    }
    0 // S_OK
}

/// Callback for when initialization is completed
extern "system" fn on_init_completed_callback() -> HRESULT {
    if let Ok(instances) = CAMERA_INSTANCES.read() {
        if let Some(camera_state) = instances.get(&0) {
            if let Ok(mut state) = camera_state.lock() {
                state.connected = true;
            }
        }
    }
    0 // S_OK
}

impl Camera {
}