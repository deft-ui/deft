use std::f32::consts::PI;
use tiny_skia::{Path, PathBuilder, Rect, Transform};

fn ellipse_arc_angle(
    pb: &mut PathBuilder,
    cx: f32,
    cy: f32,
    a: f32,
    b: f32,
    start_angle: f32,
    end_angle: f32,
    clockwise: bool,
) {
    ellipse_arc(
        pb,
        cx,
        cy,
        a,
        b,
        start_angle * PI / 180.0,
        end_angle * PI / 180.0,
        clockwise,
    );
}

fn ellipse_arc(
    pb: &mut PathBuilder,
    cx: f32,
    cy: f32,
    a: f32,
    b: f32,
    start_angle: f32,
    end_angle: f32,
    clockwise: bool,
) {
    let mut angle = end_angle - start_angle;
    if clockwise {
        if angle > 0.0 {
            angle -= 2.0 * PI;
        }
    } else {
        if angle < 0.0 {
            angle += 2.0 * PI;
        }
    }

    let segments = f32::ceil(f32::abs(angle) / (PI / 4.0));
    let segment_angle = angle / segments;

    let mut current_angle = start_angle;
    let mut x0 = cx + a * f32::cos(current_angle);
    let mut y0 = cy + b * f32::sin(current_angle);

    for _ in 0..segments as usize {
        let next_angle = current_angle + segment_angle;
        let x3 = cx + a * f32::cos(next_angle);
        let y3 = cy + b * f32::sin(next_angle);

        let k = (4.0 / 3.0) * f32::tan(segment_angle / 4.0);

        let x1 = x0 + k * (-a * f32::sin(current_angle));
        let y1 = y0 + k * (b * f32::cos(current_angle));
        let x2 = x3 - k * (-a * f32::sin(next_angle));
        let y2 = y3 - k * (b * f32::cos(next_angle));

        pb.cubic_to(x1, y1, x2, y2, x3, y3);

        current_angle = next_angle;
        x0 = x3;
        y0 = y3;
    }
}

fn resolve_x(a: f32, b: f32, c: f32) -> f32 {
    if a == 0.0 {
        return c / b;
    }
    let sq = b * b / (4.0 * a * a) - c / a;
    -b / (2.0 * a) - f32::sqrt(sq)
}

fn compute_out_cross(left_k: f32, left_radius: f32) -> (f32, f32) {
    let a = left_k * left_k + 1.0;
    let b = -2.0 * (left_k + 1.0) * left_radius;
    let c = left_radius * left_radius;
    let x = resolve_x(a, b, c);
    let y = left_k * x;
    (x, y)
}

fn compute_inner_cross(k: f32, radius: f32, border_width: [f32; 2]) -> (f32, f32) {
    let m = radius - border_width[0];
    let n = radius - border_width[1];
    let a = n * n + k * k * m * m;
    let b = -2.0 * (n * n * radius + radius * k * m * m);
    let c = radius * radius * n * n + radius * radius * m * m - m * m * n * n;
    let x = resolve_x(a, b, c);
    let y = k * x;
    (x, y)
}

fn build_corner(border_width: [f32; 2], radius: f32, transform: Option<Transform>) -> Option<Path> {
    let left_radius = radius;
    let left_k = border_width[1] / border_width[0];
    let (x, y) = compute_out_cross(left_k, left_radius);
    let (ix, iy) = compute_inner_cross(left_k, left_radius, border_width);
    let mut pb = PathBuilder::new();
    pb.move_to(x, y);

    let end_angle = 1.5 * PI;
    let start_angle = end_angle - f32::atan((left_radius - x) / (left_radius - y));
    ellipse_arc(
        &mut pb,
        left_radius,
        left_radius,
        left_radius,
        left_radius,
        start_angle,
        end_angle,
        false,
    );

    if radius > border_width[0] && radius > border_width[1] {
        pb.line_to(left_radius, border_width[1]);
        let start_deg = 1.5 * PI;
        let end_deg = 1.5 * PI - f32::atan((left_radius - ix) / (left_radius - iy));
        ellipse_arc(
            &mut pb,
            left_radius,
            left_radius,
            left_radius - border_width[0],
            left_radius - border_width[1],
            start_deg,
            end_deg,
            true,
        );
    } else {
        if radius > border_width[0] {
            pb.line_to(left_radius, border_width[1]);
        } else if radius > border_width[1] {
            pb.line_to(left_radius, 0.0);
        } else {
            pb.line_to(border_width[0], 0.0);
        }
        pb.line_to(border_width[0], border_width[1]);
    }

    pb.line_to(x, y);
    pb.close();
    if let Some(p) = pb.finish() {
        if let Some(transform) = transform {
            p.transform(transform)
        } else {
            Some(p)
        }
    } else {
        None
    }
}

fn build_line(x: f32, y: f32, width: f32, height: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    pb.push_rect(Rect::from_xywh(x, y, width, height)?);
    pb.finish()
}

fn concat(paths: &[Option<Path>]) -> Option<Path> {
    let mut pb = PathBuilder::new();
    for p in paths {
        if let Some(p) = p {
            pb.push_path(&p);
        }
    }
    pb.finish()
}

pub fn build_rect_with_radius(radius: [f32; 4], width: f32, height: f32) -> Option<Path> {
    let mut p = PathBuilder::new();
    if radius[0] == 0.0
        && radius[0] == radius[1]
        && radius[1] == radius[2]
        && radius[2] == radius[3]
    {
        let rect = Rect::from_xywh(0.0, 0.0, width, height)?;
        p.push_rect(rect);
        return p.finish();
    }

    p.move_to(0.0, radius[0]);
    ellipse_arc_angle(
        &mut p, radius[0], radius[0], radius[0], radius[0], 180.0, 270.0, false,
    );

    p.line_to(width - radius[1], 0.0);
    ellipse_arc_angle(
        &mut p,
        width - radius[1],
        radius[1],
        radius[1],
        radius[1],
        270.0,
        360.0,
        false,
    );

    p.line_to(width, height - radius[2]);
    ellipse_arc_angle(
        &mut p,
        width - radius[2],
        height - radius[2],
        radius[2],
        radius[2],
        0.0,
        90.0,
        false,
    );

    p.line_to(radius[3], height);
    ellipse_arc_angle(
        &mut p,
        radius[3],
        height - radius[3],
        radius[3],
        radius[3],
        90.0,
        180.0,
        false,
    );

    p.close();
    p.finish()
}

pub fn build_border_paths(
    border_width: [f32; 4],
    radius: [f32; 4],
    width: f32,
    height: f32,
) -> (Option<Path>, Option<Path>, Option<Path>, Option<Path>) {
    let [bt, br, bb, bl] = border_width;
    //Top
    let top_transform1 = None;
    let top_transform2 = Some(Transform::from_scale(-1.0, 1.0).post_translate(width, 0.0));
    let top_corner_1 = build_corner([bl, bt], radius[0], top_transform1);
    let top_corner_2 = build_corner([br, bt], radius[1], top_transform2);
    let top_line_space1 = f32::max(radius[0], border_width[3]);
    let top_line_space2 = f32::max(radius[1], border_width[1]);
    let top_line = build_line(
        top_line_space1,
        0.0,
        width - top_line_space1 - top_line_space2,
        bt,
    );
    let top = concat(&[top_corner_1, top_corner_2, top_line]);

    //Right
    let right_transform1 = Transform::from_rotate(90.0).post_translate(width, 0.0);
    let right_transform2 = Transform::from_scale(-1.0, 1.0)
        .post_translate(height, 0.0)
        .post_concat(right_transform1);
    let right_corner1 = build_corner([bt, br], radius[1], Some(right_transform1));
    let right_corner2 = build_corner([bb, br], radius[2], Some(right_transform2));
    let right_line_space1 = f32::max(radius[1], border_width[0]);
    let right_line_space2 = f32::max(radius[2], border_width[2]);
    let right_line = build_line(
        width - br,
        right_line_space1,
        br,
        height - right_line_space1 - right_line_space2,
    );
    let right = concat(&[right_corner1, right_corner2, right_line]);

    //Bottom
    let bottom_transform1 = Transform::from_rotate(180.0).post_translate(width, height);
    let bottom_transform2 = Transform::from_scale(-1.0, 1.0)
        .post_translate(width, 0.0)
        .post_concat(bottom_transform1);
    let bottom_corner1 = build_corner([br, bb], radius[2], Some(bottom_transform1));
    let bottom_corner2 = build_corner([bl, bb], radius[3], Some(bottom_transform2));
    let bottom_line_space1 = f32::max(radius[2], border_width[1]);
    let bottom_line_space2 = f32::max(radius[3], border_width[3]);
    let bottom_line = build_line(
        bottom_line_space2,
        height - bb,
        width - bottom_line_space1 - bottom_line_space2,
        bb,
    );
    let bottom = concat(&[bottom_corner1, bottom_corner2, bottom_line]);

    //Left
    let left_transform1 = Transform::from_rotate(270.0).post_translate(0.0, height);
    let left_transform2 = Transform::from_scale(-1.0, 1.0)
        .post_translate(height, 0.0)
        .post_concat(left_transform1);
    let left_corner1 = build_corner([bb, bl], radius[3], Some(left_transform1));
    let left_corner2 = build_corner([bt, bl], radius[0], Some(left_transform2));
    let left_line_space1 = f32::max(radius[3], border_width[2]);
    let left_line_space2 = f32::max(radius[0], border_width[0]);
    let left_line = build_line(
        0.0,
        left_line_space2,
        bl,
        height - left_line_space1 - left_line_space2,
    );
    let left = concat(&[left_corner1, left_corner2, left_line]);

    (top, right, bottom, left)
}
