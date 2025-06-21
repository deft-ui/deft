use resvg::tiny_skia;
use resvg::usvg::tiny_skia_path::PathVerb;
use skia_safe::{Path, PathFillType, Point};

pub fn tiny_path_to_skia_path(tiny_path: &tiny_skia::Path) -> Path {
    let tiny_verbs = tiny_path.verbs();
    let tiny_points = tiny_path.points();
    let mut verbs = Vec::new();
    let mut points = Vec::new();
    for tv in tiny_verbs {
        let v: u8 = match tv {
            PathVerb::Move => 0,
            PathVerb::Line => 1,
            PathVerb::Quad => 2,
            PathVerb::Cubic => 4,
            PathVerb::Close => 5,
        };
        verbs.push(v);
    }
    for p in tiny_points {
        let sp = Point::new(p.x, p.y);
        points.push(sp);
    }
    Path::new_from(&points, &verbs, &vec![], PathFillType::Winding, None)
}
