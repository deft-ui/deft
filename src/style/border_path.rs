use tiny_skia::Path;
use crate::render::cssborder::{build_border_paths, build_rect_with_radius};

pub struct BorderPath {
    box_width: f32,
    box_height: f32,
    radius: [f32; 4],
    widths: [f32; 4],
    path: Option<[Option<Path>; 4]>,
    box_path: Option<Option<Path>>,
}

impl BorderPath {
    pub fn is_same(&self, other: &BorderPath) -> bool {
        self.box_width == other.box_width
            && self.box_height == other.box_height
            && self.radius == other.radius
            && self.widths == other.widths
    }

    pub fn new(box_width: f32, box_height: f32, radius: [f32; 4], widths: [f32; 4]) -> Self {
        Self {
            box_width,
            box_height,
            radius,
            widths,
            path: None,
            box_path: None,
        }
    }

    pub fn get_box_path(&mut self) -> &Option<Path> {
        if self.box_path.is_none() {
            let p = build_rect_with_radius(self.radius, self.box_width, self.box_height);
            self.box_path = Some(p);
        }
        self.box_path.as_ref().unwrap()
    }

    pub fn get_paths(&mut self) -> &[Option<Path>; 4] {
        if self.path.is_none() {
            let (t, r, b, l) =
                build_border_paths(self.widths, self.radius, self.box_width, self.box_height);
            self.path = Some([t, r, b, l]);
        }
        self.path.as_ref().unwrap()
    }

    fn has_border(&self) -> bool {
        self.widths.iter().any(|&w| w != 0.0)
    }
}
