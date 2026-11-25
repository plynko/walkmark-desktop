use std::{path::Path, sync::OnceLock};

use image::{Rgba, RgbaImage};

use crate::error::{AppError, Result};
use crate::models::{Annotation, AnnotationKind, AnnotationStroke, Rect};

/// Walkmark's default accent when no colour is stored on the annotation.
const HIGHLIGHT: Rgba<u8> = Rgba([99, 102, 241, 255]);
const REDACT_FILL: Rgba<u8> = Rgba([24, 24, 27, 255]);
const TINT_FILL: Rgba<u8> = Rgba([245, 158, 11, 255]);
const ARROW_COLOR: Rgba<u8> = Rgba([239, 68, 68, 255]);

pub fn load(path: &Path) -> Result<RgbaImage> {
    Ok(image::open(path)?.to_rgba8())
}

pub fn load_screenshot(path: &Path) -> Result<RgbaImage> {
    let reader = image::ImageReader::open(path)?.with_guessed_format()?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
    ) {
        return Err(AppError::Invalid("Choose a PNG or JPEG screenshot.".into()));
    }
    Ok(reader.decode()?.to_rgba8())
}

pub fn fit_width(image: RgbaImage, max_w: u32) -> RgbaImage {
    if max_w == 0 || image.width() <= max_w {
        return image;
    }
    let h = (image.height() as f32 * max_w as f32 / image.width() as f32).round() as u32;
    image::imageops::resize(
        &image,
        max_w,
        h.max(1),
        image::imageops::FilterType::Lanczos3,
    )
}

/// Burn annotations into the pixels.
///
/// This happens on every export path (and before anything is sent to Gemini)
/// so a redacted region is genuinely gone, not just hidden by a CSS overlay.
pub fn apply_annotations(mut image: RgbaImage, annotations: &[Annotation]) -> RgbaImage {
    let (w, h) = (image.width(), image.height());
    for ann in annotations {
        let (x, y, rw, rh) = ann.rect.to_pixels(w, h);
        match ann.kind {
            AnnotationKind::Mouse => draw_mouse(&mut image, ann.rect),
            AnnotationKind::Arrow => draw_arrow(&mut image, ann, (x, y, rw, rh)),
            _ if rw == 0 || rh == 0 => continue,
            AnnotationKind::Blur => mosaic(&mut image, x, y, rw, rh),
            AnnotationKind::Redact => {
                let color = annotation_color(ann, REDACT_FILL);
                fill(&mut image, x, y, rw, rh, color);
            }
            AnnotationKind::Highlight => {
                let color = annotation_color(ann, HIGHLIGHT);
                let fill = Rgba([color.0[0], color.0[1], color.0[2], 40]);
                blend_fill(&mut image, x, y, rw, rh, fill);
                let stroke = highlight_stroke(w, ann.stroke);
                outline(&mut image, x, y, rw, rh, color, stroke);
            }
            AnnotationKind::Tint => {
                let mut color = annotation_color(ann, TINT_FILL);
                color.0[3] = 0x55;
                blend_fill(&mut image, x, y, rw, rh, color);
            }
        }
    }
    image
}

fn draw_arrow(image: &mut RgbaImage, ann: &Annotation, bounds: (u32, u32, u32, u32)) {
    let (x, y, w, h) = bounds;
    let direction = ann.arrow.unwrap_or_default();
    let from = (
        (if direction.reverse_x { x + w } else { x }) as f32,
        (if direction.reverse_y { y + h } else { y }) as f32,
    );
    let tip = (
        (if direction.reverse_x { x } else { x + w }) as f32,
        (if direction.reverse_y { y } else { y + h }) as f32,
    );
    let dx = tip.0 - from.0;
    let dy = tip.1 - from.1;
    let distance = dx.hypot(dy);
    if distance == 0.0 {
        return;
    }
    // Keep these proportions in sync with ArrowShape.svelte.
    let stroke = (image.width() as f32 * 0.0035).max(1.0);
    let color = annotation_color(ann, ARROW_COLOR);
    let head = (distance * 0.38).min(stroke * 4.5);
    let spread = head * 0.48;
    let (ux, uy) = (dx / distance, dy / distance);
    let base = (tip.0 - ux * head, tip.1 - uy * head);
    paint_segment(image, from, tip, stroke, color);
    paint_segment(
        image,
        (base.0 - uy * spread, base.1 + ux * spread),
        tip,
        stroke,
        color,
    );
    paint_segment(
        image,
        (base.0 + uy * spread, base.1 - ux * spread),
        tip,
        stroke,
        color,
    );
}

fn paint_segment(
    image: &mut RgbaImage,
    from: (f32, f32),
    to: (f32, f32),
    stroke: f32,
    color: Rgba<u8>,
) {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length_squared = dx * dx + dy * dy;
    if length_squared == 0.0 {
        return;
    }
    let radius = stroke / 2.0 + 0.5;
    let y0 = (from.1.min(to.1) - radius).floor() as u32;
    let y1 = ((from.1.max(to.1) + radius).ceil() as u32).min(image.height());
    for y in y0..y1 {
        let py = y as f32 + 0.5 - from.1;
        // Only visit columns that could intersect the stroke in this row.
        let (a, b) = if dy == 0.0 {
            (0.0, 1.0)
        } else {
            (
                ((py - radius) / dy).clamp(0.0, 1.0),
                ((py + radius) / dy).clamp(0.0, 1.0),
            )
        };
        let (xa, xb) = (from.0 + a * dx, from.0 + b * dx);
        let x0 = (xa.min(xb) - radius).floor() as u32;
        let x1 = ((xa.max(xb) + radius).ceil() as u32).min(image.width());
        for x in x0..x1 {
            let px = x as f32 + 0.5 - from.0;
            let t = ((px * dx + py * dy) / length_squared).clamp(0.0, 1.0);
            let coverage = (radius - (px - t * dx).hypot(py - t * dy)).clamp(0.0, 1.0);
            if coverage > 0.0 {
                let pixel = image.get_pixel_mut(x, y);
                for channel in 0..3 {
                    pixel.0[channel] = lerp(pixel.0[channel], color.0[channel], coverage);
                }
            }
        }
    }
}

fn draw_mouse(image: &mut RgbaImage, rect: Rect) {
    static MARKER: OnceLock<RgbaImage> = OnceLock::new();
    let marker = MARKER.get_or_init(|| {
        image::load_from_memory_with_format(
            include_bytes!("../../src/lib/assets/mouse-marker.png"),
            image::ImageFormat::Png,
        )
        .expect("bundled cursor marker is a valid PNG")
        .into_rgba8()
    });
    let (w, h) = (image.width() as f32, image.height() as f32);
    let size = (rect.w * w).min(rect.h * h).min(w.min(h) * 0.5).round() as u32;
    if size == 0 {
        return;
    }
    let left = ((rect.x + rect.w / 2.0) * w - size as f32 / 2.0).round() as i64;
    let top = ((rect.y + rect.h / 2.0) * h - size as f32 / 2.0).round() as i64;
    let marker = image::imageops::resize(marker, size, size, image::imageops::FilterType::Triangle);
    image::imageops::overlay(image, &marker, left, top);
}

/// Render edits in original frame coordinates before cutting the visible region.
pub fn render_frame(image: RgbaImage, annotations: &[Annotation], crop: Option<Rect>) -> RgbaImage {
    let mut image = apply_annotations(image, annotations);
    if let Some(crop) = crop {
        let (x, y, w, h) = crop.to_pixels(image.width(), image.height());
        if w > 0 && h > 0 {
            return image::imageops::crop(&mut image, x, y, w, h).to_image();
        }
    }
    image
}

fn highlight_stroke(img_w: u32, stroke: Option<AnnotationStroke>) -> u32 {
    let base = (img_w as f32 / 400.0).round().clamp(2.0, 8.0) as u32;
    match stroke.unwrap_or(AnnotationStroke::Medium) {
        AnnotationStroke::Thin => base.saturating_sub(1).max(1),
        AnnotationStroke::Medium => base,
        AnnotationStroke::Thick => base + 2,
    }
}

fn annotation_color(ann: &Annotation, fallback: Rgba<u8>) -> Rgba<u8> {
    ann.color
        .as_deref()
        .and_then(parse_hex_color)
        .unwrap_or(fallback)
}

fn parse_hex_color(hex: &str) -> Option<Rgba<u8>> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Rgba([r, g, b, 255]))
}

/// Mosaic rather than a gaussian blur: blurred text can sometimes be recovered,
/// and a coarse mosaic reads unambiguously as "deliberately hidden".
fn mosaic(image: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32) {
    let block = (w.min(h) as f32 / 6.0).round().clamp(8.0, 40.0) as u32;
    let mut by = y;
    while by < y + h {
        let mut bx = x;
        while bx < x + w {
            let bw = block.min(x + w - bx);
            let bh = block.min(y + h - by);
            let (mut r, mut g, mut b, mut n) = (0u32, 0u32, 0u32, 0u32);
            for py in by..by + bh {
                for px in bx..bx + bw {
                    let p = image.get_pixel(px, py).0;
                    r += p[0] as u32;
                    g += p[1] as u32;
                    b += p[2] as u32;
                    n += 1;
                }
            }
            if let (Some(r), Some(g), Some(b)) =
                (r.checked_div(n), g.checked_div(n), b.checked_div(n))
            {
                let avg = Rgba([r as u8, g as u8, b as u8, 255]);
                for py in by..by + bh {
                    for px in bx..bx + bw {
                        image.put_pixel(px, py, avg);
                    }
                }
            }
            bx += block;
        }
        by += block;
    }
}

fn fill(image: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    for py in y..(y + h).min(image.height()) {
        for px in x..(x + w).min(image.width()) {
            image.put_pixel(px, py, color);
        }
    }
}

fn blend_fill(image: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, tint: Rgba<u8>) {
    let alpha = tint.0[3] as f32 / 255.0;
    for py in y..(y + h).min(image.height()) {
        for px in x..(x + w).min(image.width()) {
            let base = image.get_pixel(px, py).0;
            let blended = Rgba([
                lerp(base[0], tint.0[0], alpha),
                lerp(base[1], tint.0[1], alpha),
                lerp(base[2], tint.0[2], alpha),
                255,
            ]);
            image.put_pixel(px, py, blended);
        }
    }
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

fn outline(image: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>, stroke: u32) {
    let (iw, ih) = (image.width(), image.height());
    for s in 0..stroke {
        let top = y.saturating_add(s);
        let bottom = (y + h).saturating_sub(s + 1);
        for px in x..(x + w).min(iw) {
            if top < ih {
                image.put_pixel(px, top, color);
            }
            if bottom < ih {
                image.put_pixel(px, bottom, color);
            }
        }
        let left = x.saturating_add(s);
        let right = (x + w).saturating_sub(s + 1);
        for py in y..(y + h).min(ih) {
            if left < iw {
                image.put_pixel(left, py, color);
            }
            if right < iw {
                image.put_pixel(right, py, color);
            }
        }
    }
}

pub fn encode_jpeg(image: &RgbaImage, quality: u8) -> Result<Vec<u8>> {
    let mut buf = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    image::DynamicImage::ImageRgba8(image.clone())
        .to_rgb8()
        .write_with_encoder(encoder)?;
    Ok(buf)
}

pub fn encode_png(image: &RgbaImage) -> Result<Vec<u8>> {
    let mut buf = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image.clone())
        .to_rgb8()
        .write_to(&mut buf, image::ImageFormat::Png)?;
    Ok(buf.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ArrowDirection;

    #[test]
    fn tint_blends_the_selected_colour_over_the_screenshot() {
        let background = Rgba([30, 60, 90, 255]);
        let image = RgbaImage::from_pixel(40, 40, background);
        let tint = Annotation {
            id: "tint".into(),
            kind: AnnotationKind::Tint,
            rect: Rect {
                x: 0.25,
                y: 0.25,
                w: 0.5,
                h: 0.5,
            },
            color: Some("#00ff00".into()),
            stroke: None,
            arrow: None,
        };
        let output = apply_annotations(image, &[tint]);
        assert_eq!(*output.get_pixel(20, 20), Rgba([20, 125, 60, 255]));
        assert_eq!(*output.get_pixel(0, 0), background);
    }

    #[test]
    fn horizontal_arrow_points_to_the_end_of_a_right_to_left_drag() {
        let background = Rgba([255, 255, 255, 255]);
        let image = RgbaImage::from_pixel(1000, 300, background);
        let arrow = Annotation {
            id: "arrow".into(),
            kind: AnnotationKind::Arrow,
            rect: Rect {
                x: 0.2,
                y: 0.5,
                w: 0.6,
                h: 0.0,
            },
            color: None,
            stroke: None,
            arrow: Some(ArrowDirection {
                reverse_x: true,
                reverse_y: false,
            }),
        };
        let output = apply_annotations(image, &[arrow]);
        assert_eq!(*output.get_pixel(400, 149), ARROW_COLOR);
        assert_ne!(*output.get_pixel(214, 156), background);
        assert_eq!(*output.get_pixel(785, 156), background);
    }

    #[test]
    fn crop_keeps_mouse_marker_at_its_original_click_position() {
        let background = Rgba([28, 48, 68, 255]);
        let image = RgbaImage::from_pixel(400, 200, background);
        let marker = Annotation {
            id: "click".into(),
            kind: AnnotationKind::Mouse,
            rect: Rect {
                x: 0.7225,
                y: 0.445,
                w: 0.055,
                h: 0.11,
            },
            color: None,
            stroke: None,
            arrow: None,
        };
        let crop = Rect {
            x: 0.5,
            y: 0.25,
            w: 0.5,
            h: 0.5,
        };

        let output = render_frame(image, &[marker], Some(crop));
        assert_eq!(output.dimensions(), (200, 100));
        assert_eq!(*output.get_pixel(0, 0), background);
        assert_ne!(*output.get_pixel(100, 50), background);
        let halo = output.get_pixel(94, 50);
        assert!(halo[0] > background[0] && halo[0] < 255);
    }

    #[test]
    fn an_enlarged_mouse_marker_is_drawn_at_its_full_size() {
        let background = Rgba([28, 48, 68, 255]);
        let image = RgbaImage::from_pixel(400, 400, background);
        // 160 px wide: well past the old limit of 15% of the image.
        let marker = Annotation {
            id: "big".into(),
            kind: AnnotationKind::Mouse,
            rect: Rect {
                x: 0.3,
                y: 0.3,
                w: 0.4,
                h: 0.4,
            },
            color: None,
            stroke: None,
            arrow: None,
        };
        let output = apply_annotations(image, &[marker]);
        // The halo reaches close to the edge of the 160 px square.
        let changed = (0..400u32)
            .filter(|&x| *output.get_pixel(x, 200) != background)
            .count();
        assert!(changed > 100, "{changed}");
    }

    #[test]
    fn highlight_uses_stored_colour_and_fill() {
        let image = RgbaImage::from_pixel(40, 40, Rgba([255, 255, 255, 255]));
        let out = apply_annotations(
            image,
            &[Annotation {
                id: "a".into(),
                kind: AnnotationKind::Highlight,
                rect: Rect {
                    x: 0.25,
                    y: 0.25,
                    w: 0.5,
                    h: 0.5,
                },
                color: Some("#ef4444".into()),
                stroke: Some(AnnotationStroke::Thick),
                arrow: None,
            }],
        );
        let edge = out.get_pixel(10, 10).0;
        assert_eq!(edge[0], 239);
        assert_eq!(edge[1], 68);
        assert_eq!(edge[2], 68);
    }

    #[test]
    fn redact_uses_stored_fill_colour() {
        let image = RgbaImage::from_pixel(20, 20, Rgba([0, 0, 0, 255]));
        let out = apply_annotations(
            image,
            &[Annotation {
                id: "a".into(),
                kind: AnnotationKind::Redact,
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 0.5,
                    h: 0.5,
                },
                color: Some("#ffffff".into()),
                stroke: None,
                arrow: None,
            }],
        );
        assert_eq!(out.get_pixel(5, 5).0, [255, 255, 255, 255]);
    }
}
