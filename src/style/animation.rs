pub struct AnimationParams {
    pub name: String,
    pub duration: f32,
    pub iteration_count: f32,
}

impl AnimationParams {
    pub fn new() -> Self {
        Self {
            name: "".to_string(),
            duration: 0.0,
            iteration_count: 1.0,
        }
    }
}
