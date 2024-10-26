use skia_bindings::{SkPaint_Style, SkPathOp};
use skia_safe::{Canvas, Color, Matrix, Paint, Path, Rect};

#[derive(Clone)]
pub enum BorderStyle {
    SOLID,
}

pub type BorderProps = (f32, Color);

#[derive(Clone)]
pub struct Border {
    pub top: Option<BorderProps>,
    pub right: Option<BorderProps>,
    pub bottom: Option<BorderProps>,
    pub left: Option<BorderProps>,
    pub bottom_left_radius: f32,
    pub bottom_right_radius: f32,
    pub top_left_radius: f32,
    pub top_right_radius: f32,
}

impl Border {
    pub fn default() -> Self {
        Self {
            top: None,
            right: None,
            bottom: None,
            left: None,
            bottom_left_radius: 0.0,
            bottom_right_radius: 0.0,
            top_left_radius: 0.0,
            top_right_radius: 0.0,
        }
    }
}

pub fn build_border_paths(border_width: [f32; 4], border_radius: [f32; 4], width: f32, height: f32) -> [Path; 4] {
    let [top_left_radius, top_right_radius, bottom_right_radius, bottom_left_radius] = border_radius;
    let top = (border_width[0], Color::new(0));
    let right = (border_width[1],  Color::new(0));
    let bottom = (border_width[2],  Color::new(0));
    let left = (border_width[3],  Color::new(0));
    let borders = vec![
        (top, left, top_left_radius, right, top_right_radius, width, height, 0.0, 0.0, 0.0),
        (right, top, top_right_radius, bottom, bottom_right_radius, height, width, 90.0, width, 0.0),
        (bottom, right, bottom_right_radius, left, bottom_left_radius, width, height, 180.0, width, height),
        (left, bottom, bottom_left_radius, top, top_left_radius, height, width, 270.0, 0.0, height),
    ];
    let mut result = Vec::new();
    for (top, left, left_radius, right, right_radius, width, height, rotate, tx, ty) in borders {
        if let Some(mut path) = draw_top_border(&top, &left, left_radius, &right, right_radius, width, height) {
            let mut matrix = Matrix::rotate_deg(rotate);
            matrix.post_translate((tx, ty));
            path.transform(&matrix);
            result.push(path);
        } else {
            result.push(Path::new());
        }
    }
    return unsafe { [
        result.get_unchecked(0).clone(),
        result.get_unchecked(1).clone(),
        result.get_unchecked(2).clone(),
        result.get_unchecked(3).clone()
    ] }
}



pub fn draw_border(canvas: &Canvas, border_width: [f32; 4], border_color: [Color; 4], border_radius: [f32; 4], width: f32, height: f32) {
    let [top_left_radius, top_right_radius, bottom_right_radius, bottom_left_radius] = border_radius;
    let top = (border_width[0], border_color[0]);
    let right = (border_width[1], border_color[1]);
    let bottom = (border_width[2], border_color[2]);
    let left = (border_width[3], border_color[3]);
    let borders = vec![
        (top, left, top_left_radius, right, top_right_radius, width, height, 0.0, 0.0, 0.0),
        (right, top, top_right_radius, bottom, bottom_right_radius, height, width, 90.0, width, 0.0),
        (bottom, right, bottom_right_radius, left, bottom_left_radius, width, height, 180.0, width, height),
        (left, bottom, bottom_left_radius, top, top_left_radius, height, width, 270.0, 0.0, height),
    ];
    for (top, left, left_radius, right, right_radius, width, height, rotate, tx, ty) in borders {
        if let Some(mut path) = draw_top_border(&top, &left, left_radius, &right, right_radius, width, height) {
            let color = top.1;
            let mut paint = Paint::default();
            paint.set_style(SkPaint_Style::Fill);
            paint.set_anti_alias(true);
            paint.set_color(color);
            let mut matrix = Matrix::rotate_deg(rotate);
            matrix.post_translate((tx, ty));
            path.transform(&matrix);

            canvas.draw_path(&path, &paint);
        }
    }
}


fn draw_top_border(top_border: &BorderProps,
                   left_border: &BorderProps, left_radius: f32,
                   right_border: &BorderProps, right_radius: f32,
                   width: f32, height: f32,
) -> Option<Path> {
    let props = top_border;
    let border_width = props.0;
    if border_width <= 0.0 {
        return None;
    }
    let mut path = Path::new();
    //let mut clip_path = Path::new();
    path.add_rect(Rect::new(0.0, 0.0, width, height), None);
    let left_border_width = left_border.0;
    let right_border_width = right_border.0;
    let mut paths = Vec::new();
    let mut clip_paths = Vec::new();
    let mut right_matrix = Matrix::scale((-1.0, 1.0));
    right_matrix.set_translate_x(width);
    if left_border_width > 0.0 {
        // left border radius
        let cp = build_border_clip(left_border_width, border_width, height);
        clip_paths.push(cp);
    }
    if right_border_width > 0.0 {
        let mut cp = build_border_clip(right_border_width, border_width, height);
        cp.transform(&right_matrix);
        clip_paths.push(cp);
    }
    let mut clip_ps = Vec::new();
    clip_ps.push(build_radius_clip(border_width, left_border_width, left_radius, width, height));
    let mut cp = build_radius_clip(border_width, right_border_width, right_radius, width, height);
    cp.transform(&right_matrix);
    clip_ps.push(cp);
    if left_radius > 0.0 || right_radius > 0.0 {
        // radius path & clip
        let mut ps = Vec::new();
        if left_radius > 0.0 {
            ps.push(build_radius_path(left_radius, width, height));
        }
        if right_radius > 0.0 {
            let mut p = build_radius_path(right_radius, width, height);
            p.transform(&right_matrix);
            ps.push(p);
        }
        paths.push(intersect_paths(ps));
    }
    clip_paths.push(intersect_paths(clip_ps));

    for p in paths {
        path = path.op(&p, SkPathOp::Intersect).unwrap_or(Path::new());
    }
    for p in clip_paths {
        path = path.op(&p, SkPathOp::Difference).unwrap_or(Path::new());
    }
    Some(path)
}

fn build_border_clip(left_border_width: f32, border_width: f32, height: f32) -> Path {
    let w = left_border_width * height / border_width;
    let mut cp = Path::new();
    cp.move_to((0.0, 0.0));
    cp.line_to((w, height));
    cp.line_to((0.0, height));
    cp.close();
    cp
}

pub fn build_rect_with_radius(radius: [f32; 4], width: f32, height: f32) -> Path {
    let mut p = Path::new();

    p.move_to((0.0, radius[0]));
    p.arc_to(Rect::new(0.0, 0.0, radius[0] * 2.0, radius[0] * 2.0), 180.0, 90.0, false);

    p.line_to((width - radius[1], 0.0));
    p.arc_to(Rect::new(width - radius[1] * 2.0, 0.0, width, radius[1] * 2.0), 270.0, 90.0, false);

    p.line_to((width, height - radius[2]));
    p.arc_to(Rect::new(width - radius[2] * 2.0, height - radius[3] * 2.0, width, height), 0.0, 90.0, false);

    p.line_to((radius[3], height));
    p.arc_to(Rect::new(0.0, height - radius[3] * 2.0, radius[3] * 2.0, height), 90.0, 90.0, false);

    p.close();
    p
}

fn build_radius_path(left_radius: f32, width: f32, height: f32) -> Path {
    let mut p = Path::new();
    p.move_to((0.0, left_radius));
    p.arc_to(Rect::new(0.0, 0.0, left_radius, left_radius), 180.0, 90.0, false);
    p.line_to((width, 0.0));
    p.line_to((width, height));
    p.line_to((0.0, height));
    p.close();
    p
}

fn build_radius_clip(border_width: f32, left_border_width: f32, left_radius: f32, width: f32, height: f32) -> Path {
    let mut p = Path::new();
    if left_radius > left_border_width && left_radius > border_width {
        let inner_arc_width = (left_radius - left_border_width) * 2.0;
        let inner_arc_height = (left_radius - border_width) * 2.0;
        let rect = Rect::new(left_border_width, border_width, left_border_width + inner_arc_width, border_width + inner_arc_height);
        p.move_to((left_border_width, left_radius));
        p.arc_to(rect, 180.0, 90.0, false);
        p.line_to((width, border_width));
        p.line_to((width, height));
        p.line_to((left_border_width, height));
        p.close();
    } else {
        p.add_rect(Rect::new(left_border_width, border_width, width, height), None);
    }
    p
}

fn intersect_paths(paths: Vec<Path>) -> Path {
    let mut path = Path::new();
    let mut first = true;
    for p in paths {
        if first {
            path = p.clone();
            first = false;
        } else {
            path = path.op(&p, SkPathOp::Intersect).unwrap_or(Path::new())
        }
    }
    path
}