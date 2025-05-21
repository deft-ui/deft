use skia_safe::Canvas;

pub trait CanvasHelper {
    fn session<F: FnOnce(&Self)>(&self, callback: F);
}

impl CanvasHelper for Canvas {
    fn session<F: FnOnce(&Self)>(&self, callback: F) {
        self.save();
        callback(&self);
        self.restore();
    }
}
