use lento::app::LentoApp;
use lento::bootstrap;

struct DefaultLentoApp {}

impl LentoApp for DefaultLentoApp {}

fn main() {
    let app = DefaultLentoApp {};
    bootstrap(Box::new(app));
}