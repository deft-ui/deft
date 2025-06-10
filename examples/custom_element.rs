#![windows_subsystem = "windows"]
use deft::app::{App, IApp};
use deft::bootstrap;
use deft::element::{register_component, Element, ElementBackend, ElementWeak};
use deft::js::js_engine::JsEngine;
use deft::render::RenderFn;
use quick_js::loader::{FsJsModuleLoader, JsModuleLoader};
use skia_safe::{Color, Paint, PaintStyle};

/// Begin Custom Element

struct HelloBackend {
    element_weak: ElementWeak,
}

impl ElementBackend for HelloBackend {
    fn create(element: &mut Element) -> Self
    where
        Self: Sized,
    {
        Self {
            element_weak: element.as_weak(),
        }
    }

    fn render(&mut self) -> RenderFn {
        let element = self.element_weak.upgrade_mut().unwrap();
        let bounds = element.get_bounds();
        let center = (bounds.width / 2.0, bounds.height / 2.0);
        let radius = f32::min(center.0, center.1);
        RenderFn::new(move |painter| {
            let mut paint = Paint::default();
            paint.set_style(PaintStyle::Fill);
            paint.set_color(Color::from_rgb(0, 80, 0));
            painter.canvas.draw_circle(center, radius, &paint);
        })
    }
}

/// End CustomElement
struct AppImpl {}

impl IApp for AppImpl {
    fn init_js_engine(&mut self, _js_engine: &mut JsEngine) {
        register_component::<HelloBackend>("hello");
    }
    fn create_module_loader(&mut self) -> Box<dyn JsModuleLoader + Send + Sync + 'static> {
        let ml = FsJsModuleLoader::new("examples/custom-element-js");
        Box::new(ml)
    }
}

fn main() {
    env_logger::init();
    let app = App::new(AppImpl {});
    bootstrap(app);
}
