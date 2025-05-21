pub trait LayoutRoot {
    fn update_layout(&mut self);
    fn should_propagate_dirty(&self) -> bool;
    // fn on_root_bounds_updated(&mut self, bounds: Rect);
}
