use std::f32::consts::PI;

use tiny_skia::{Color, LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::usage::Limit;

const SIZE: u32 = 32;
const SCALE: f32 = SIZE as f32 / 18.0;
const SPARK: [u8; 3] = [255, 149, 0];
const CRITICAL: [u8; 3] = [255, 69, 58];

pub struct Rgba {
    pub data: Vec<u8>,
    pub size: u32,
}

pub fn render(weekly: Option<&Limit>, session: Option<&Limit>, foreground: [u8; 3]) -> Rgba {
    let mut pixmap = Pixmap::new(SIZE, SIZE).expect("non-zero icon size");
    let base = rgb(foreground, 255);
    let track = rgb(foreground, 77);
    let center = SIZE as f32 / 2.0;

    for (radius, limit) in [(7.9, weekly), (4.9, session)] {
        let radius = radius * SCALE;
        stroke(&mut pixmap, circle(center, radius), track, 1.8 * SCALE);
        if let Some(limit) = limit.filter(|limit| limit.percent > 0) {
            let color = if limit.is_critical() {
                rgb(CRITICAL, 255)
            } else {
                base
            };
            stroke(
                &mut pixmap,
                arc(center, radius, limit.percent.min(100) as f32 / 100.0),
                color,
                1.8 * SCALE,
            );
        }
    }
    spark(&mut pixmap, center, 2.9 * SCALE);

    let data = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();
    Rgba { data, size: SIZE }
}

fn stroke(pixmap: &mut Pixmap, path: Option<tiny_skia::Path>, color: Color, width: f32) {
    let Some(path) = path else { return };
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;
    let stroke = Stroke {
        width,
        line_cap: LineCap::Round,
        ..Stroke::default()
    };
    pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

fn circle(center: f32, radius: f32) -> Option<tiny_skia::Path> {
    PathBuilder::from_circle(center, center, radius)
}

fn arc(center: f32, radius: f32, fraction: f32) -> Option<tiny_skia::Path> {
    let steps = (fraction * 96.0).ceil().max(2.0) as usize;
    let mut builder = PathBuilder::new();
    for step in 0..=steps {
        let angle = -PI / 2.0 + 2.0 * PI * fraction * step as f32 / steps as f32;
        let (x, y) = (center + radius * angle.cos(), center + radius * angle.sin());
        if step == 0 {
            builder.move_to(x, y);
        } else {
            builder.line_to(x, y);
        }
    }
    builder.finish()
}

fn spark(pixmap: &mut Pixmap, center: f32, radius: f32) {
    let mut builder = PathBuilder::new();
    for ray in 0..12 {
        let angle = ray as f32 / 12.0 * 2.0 * PI - PI / 2.0;
        let length = if ray % 2 == 0 { radius } else { radius * 0.62 };
        builder.move_to(
            center + angle.cos() * radius * 0.18,
            center + angle.sin() * radius * 0.18,
        );
        builder.line_to(center + angle.cos() * length, center + angle.sin() * length);
    }
    stroke(pixmap, builder.finish(), rgb(SPARK, 255), radius * 0.32);
}

fn rgb([r, g, b]: [u8; 3], alpha: u8) -> Color {
    Color::from_rgba8(r, g, b, alpha)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_square_rgba() {
        let icon = render(None, None, [255, 255, 255]);
        assert_eq!(icon.data.len(), (icon.size * icon.size * 4) as usize);
        assert!(icon.data.chunks(4).any(|pixel| pixel[3] > 0));
    }
}
