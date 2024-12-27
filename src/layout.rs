use skia_safe::Rect;

pub trait LayoutRoot {
    fn mark_layout_dirty(&mut self);
    fn on_root_bounds_updated(&mut self, bounds: Rect);
}