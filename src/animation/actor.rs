pub enum AnimationAction {
    Progress(f32),
    Stop,
}
pub trait AnimationActor {
    fn apply_animation(&mut self, progress: f32, stop: &mut bool);
    fn stop(&mut self) {}
}
