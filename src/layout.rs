use skia_safe::Rect;

pub trait LayoutRoot {
    fn update_layout(&mut self);
    // fn on_root_bounds_updated(&mut self, bounds: Rect);
}