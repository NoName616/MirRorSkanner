use std::time::{Duration, Instant};

use rand::{rngs::StdRng, Rng, SeedableRng};

use super::backend::{
    CameraBackend, CameraError, CameraFrameMetadata, FrameDimensions, RawFrame,
    TemperatureConversion,
};

pub struct MockCameraBackend {
    dims: FrameDimensions,
    rng: StdRng,
    frame_counter: u32,
    start: Instant,
}

impl MockCameraBackend {
    pub fn new() -> Self {
        Self {
            dims: FrameDimensions {
                width: 640,
                height: 480,
                depth: 16,
            },
            rng: StdRng::seed_from_u64(0xDEADBEEF),
            frame_counter: 0,
            start: Instant::now(),
        }
    }
}

impl CameraBackend for MockCameraBackend {
    fn prepare(&mut self, _instance_name: Option<&str>) -> Result<FrameDimensions, CameraError> {
        Ok(self.dims)
    }

    fn grab_raw_frame(&mut self, _timeout: Duration) -> Result<RawFrame, CameraError> {
        let pixel_count = self.dims.pixel_count();
        if pixel_count == 0 {
            return Err(CameraError::InvalidFrame);
        }

        let mut pixels = Vec::with_capacity(pixel_count);
        let base_temp: f32 = 3200.0 + (self.frame_counter as f32 % 50.0);

        for idx in 0..pixel_count {
            let fluctuation: f32 = self.rng.gen_range(-50.0..50.0);
            let value = (base_temp + fluctuation + (idx % 100) as f32 * 0.5).max(0.0);
            pixels.push(value as u16);
        }

        let now = Instant::now();
        let metadata = CameraFrameMetadata {
            frame_counter: self.frame_counter,
            hardware_counter: self.frame_counter,
            timestamp_us: (now - self.start).as_micros() as i64,
            flag_state: 0,
        };

        self.frame_counter = self.frame_counter.wrapping_add(1);

        Ok(RawFrame { pixels, metadata })
    }

    fn temperature_conversion(&self) -> TemperatureConversion {
        TemperatureConversion::default()
    }

    fn chip_temperature(&self) -> Option<f32> {
        Some(45.0 + (self.frame_counter % 10) as f32 * 0.5)
    }

    fn flag_temperature(&self) -> Option<f32> {
        Some(42.0 + (self.frame_counter % 8) as f32 * 0.4)
    }

    fn serial_number(&self) -> Option<u32> {
        Some(0xDEADBEEF)
    }

    fn backend_name(&self) -> &'static str {
        "Mock Thermal Camera"
    }
}
