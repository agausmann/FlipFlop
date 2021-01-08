#[derive(Default)]
pub struct Config {
    pub camera: CameraConfig,
}

pub struct CameraConfig {
    pub pan_speed: f32,
    pub zoom_step: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            pan_speed: 30.0,
            zoom_step: 0.05,
            min_zoom: 0.25,
            max_zoom: 4.0,
        }
    }
}
