#![windows_subsystem = "windows"]

use deft_macros::{widget, js_methods};
use deft::app::{App, IApp};
use deft::{bootstrap, js_module};
use deft::element::{Element, Widget, ElementDelegate, ElementWeak};
use deft::js::js_engine::JsEngine;
use deft::render::RenderFn;
use quick_js::loader::{FsJsModuleLoader, JsModuleLoader};
use skia_safe::{Color, Paint, PaintStyle};

/// Begin Custom Element

#[widget]
struct HelloWidget {

}

impl Widget for HelloWidget {}

js_module!(HelloWidget, include_str!("./custom_widget.js"));

#[js_methods]
impl HelloWidget {

    #[js_func]
    pub fn create() -> Self {
        let mut el = Element::new("hello");
        el.set_delegate(HelloElementDelegate {
            element_weak: el.as_weak(),
        });
        Self {
            el,
        }
    }
}

struct HelloElementDelegate {
    element_weak: ElementWeak,
}


impl ElementDelegate for HelloElementDelegate {

    fn render(&mut self) -> RenderFn {
        let element = self.element_weak.upgrade().unwrap();
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
    fn init_js_engine(&mut self, js_engine: &mut JsEngine) {
        js_engine.register_module::<HelloWidget>("custom_widget").unwrap();
    }
    fn create_module_loader(&mut self) -> Box<dyn JsModuleLoader + Send + Sync + 'static> {
        let ml = FsJsModuleLoader::new("examples/custom-widget-js");
        Box::new(ml)
    }
}

fn main() {
    env_logger::init();
    let app = App::new(AppImpl {});
    bootstrap(app);
}
