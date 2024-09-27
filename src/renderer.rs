use measure_time::print_time;
use skia_bindings::SkPaint_Style;
use skia_safe::{Canvas, Color, Paint, Surface, surfaces};
use skia_window::skia_window::SkiaWindow;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopBuilder};
use winit::window::{WindowAttributes, WindowId};
use yoga::Direction;
use crate::app::AppEvent;
use crate::border::build_border_paths;
use crate::canvas_util::CanvasHelper;
use crate::style::{Style, StyleNode, StylePropertyKey, StylePropertyValue};


//TODO remove
pub struct CpuRenderer {
    surface: Surface,
}

impl CpuRenderer {
    pub fn new(width: i32, height: i32) -> Self {
        let surface = surfaces::raster_n32_premul((width, height)).unwrap();
        CpuRenderer {
            surface,
        }
    }

    pub fn surface(&mut self) -> &mut Surface {
        &mut self.surface
    }

    pub fn canvas(&mut self) -> &Canvas {
        self.surface.canvas()
    }

}

pub fn test_border(canvas: &Canvas) {
    print_time!("draw border time");
    let mut style = StyleNode::new();
    style.border_radius = [10.0, 10.0, 10.0, 10.0];
    style.set_style(&StylePropertyKey::BorderLeft, &StylePropertyValue::String("1 #ccc".to_string()));
    style.set_style(&StylePropertyKey::BorderRight, &StylePropertyValue::String("1 #ccc".to_string()));
    style.set_style(&StylePropertyKey::BorderTop, &StylePropertyValue::String("1 #ccc".to_string()));
    style.set_style(&StylePropertyKey::BorderBottom, &StylePropertyValue::String("1 #ccc".to_string()));
    style.calculate_layout(70.0, 16.0, Direction::LTR);
    let width = style.get_layout_width();
    let height = style.get_layout_height();
    println!("size: {} x {}", width, height);
    for i in 0..1000 {
        let y = (i / 20) * 20;
        let x = i % 20 * 80;
        let ps = style.get_border_paths();
        canvas.session(move |canvas| {
            canvas.translate((x, y));
            for p in &ps {
                let mut paint = Paint::default();
                paint.set_style(SkPaint_Style::Fill);
                paint.set_anti_alias(true);
                paint.set_color(Color::from_rgb(255, 255, 255));
                canvas.draw_path(p, &paint);
            }
        });
    }
}

#[test]
fn test_border_performance() {
    let mut render = CpuRenderer::new(1024, 1024);
    let canvas = render.canvas();
    test_border(canvas);
}
