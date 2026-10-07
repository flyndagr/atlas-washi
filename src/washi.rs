//! Original procedural paper and ink artwork; no downloaded textures.
use eframe::egui::*;
pub const INK: Color32 = Color32::from_rgb(54, 53, 43);
pub const MUTED: Color32 = Color32::from_rgb(96, 88, 72);
pub const PAPER: Color32 = Color32::from_rgb(248, 243, 226);
pub const PANEL: Color32 = Color32::from_rgb(237, 229, 207);
pub const LINE: Color32 = Color32::from_rgb(219, 209, 182);
pub const ACCENT: Color32 = Color32::from_rgb(160, 68, 49);
pub const GREEN: Color32 = Color32::from_rgb(80, 92, 60);
pub fn serif(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Serif".into()))
}
pub fn pen(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Pen".into()))
}
pub fn script(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Script".into()))
}
pub fn fonts() -> FontDefinitions {
    let mut f = FontDefinitions::default();
    let faces: [(&str, &[u8]); 5] = [
        ("Inter", include_bytes!("../assets/Inter-Regular.ttf")),
        ("Serif", include_bytes!("../assets/CormorantGaramond.ttf")),
        (
            "Pen",
            include_bytes!("../assets/CormorantGaramond-Italic.ttf"),
        ),
        ("Script", include_bytes!("../assets/Allura-Regular.ttf")),
        ("Japanese", include_bytes!("../assets/ShipporiMincho.ttf")),
    ];
    for (name, bytes) in faces {
        let mut data = FontData::from_static(bytes);
        if matches!(name, "Serif" | "Pen") {
            data.tweak.coords = epaint::text::VariationCoords::new([(b"wght", 450.)]);
        }
        f.font_data.insert(name.into(), data.into());
        f.families.insert(
            FontFamily::Name(name.into()),
            vec![name.into(), "Japanese".into(), "Inter".into()],
        );
    }
    f.families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "Inter".into());
    f.families
        .entry(FontFamily::Proportional)
        .or_default()
        .push("Japanese".into());
    f
}
#[derive(Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub enum PaperKind {
    #[default]
    Kozo,
    Ivory,
    Mist,
    Tea,
}
impl PaperKind {
    pub const ALL: [Self; 4] = [Self::Kozo, Self::Ivory, Self::Mist, Self::Tea];
    pub fn name(self) -> &'static str {
        match self {
            Self::Kozo => "Kozo · Sumi",
            Self::Ivory => "Ivory · Sepia",
            Self::Mist => "Mist · Indigo",
            Self::Tea => "Tea · Moss",
        }
    }
    pub fn sheet(self) -> Color32 {
        match self {
            Self::Kozo => Color32::from_rgb(249, 244, 227),
            Self::Ivory => Color32::from_rgb(253, 240, 216),
            Self::Mist => Color32::from_rgb(235, 241, 236),
            Self::Tea => Color32::from_rgb(235, 226, 201),
        }
    }
    pub fn ink(self) -> Color32 {
        match self {
            Self::Kozo => INK,
            Self::Ivory => Color32::from_rgb(88, 58, 44),
            Self::Mist => Color32::from_rgb(42, 65, 89),
            Self::Tea => Color32::from_rgb(61, 76, 52),
        }
    }
}
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub kind: PaperKind,
    pub grain: f32,
    pub fountain: bool,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            kind: PaperKind::Kozo,
            grain: 0.7,
            fountain: true,
        }
    }
}
// Periodic smooth noise gives the pulp broad, irregular density without visible tile seams.
fn pulp(x: f32, y: f32, cells: u32, salt: u32) -> f32 {
    let hash = |x: u32, y: u32| {
        let mut n =
            (x % cells).wrapping_mul(374761393) ^ (y % cells).wrapping_mul(668265263) ^ salt;
        n = (n ^ (n >> 13)).wrapping_mul(1274126177);
        (n ^ (n >> 16)) as f32 / u32::MAX as f32
    };
    let x = x * cells as f32;
    let y = y * cells as f32;
    let ix = x.floor() as u32;
    let iy = y.floor() as u32;
    let smooth = |t: f32| t * t * (3. - 2. * t);
    let u = smooth(x.fract());
    let v = smooth(y.fract());
    let top = hash(ix, iy) * (1. - u) + hash(ix + 1, iy) * u;
    let bottom = hash(ix, iy + 1) * (1. - u) + hash(ix + 1, iy + 1) * u;
    top * (1. - v) + bottom * v
}
pub fn texture(ctx: &Context, grain: f32) -> TextureHandle {
    let n = 1024usize;
    let strength = grain.clamp(0., 1.);
    let mut seed = 712367u32;
    let mut rand = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as f32 / u32::MAX as f32
    };
    let mut pixels = Vec::with_capacity(n * n);
    for y in 0..n {
        for x in 0..n {
            let xx = x as f32 / n as f32;
            let yy = y as f32 / n as f32;
            let cloud = pulp(xx, yy, 8, 93) * 0.55
                + pulp(xx, yy, 31, 721) * 0.30
                + pulp(xx, yy, 113, 8123) * 0.15;
            let density = ((cloud - 0.33) * 35. + rand() * 8.).max(0.);
            pixels.push(Color32::from_rgba_unmultiplied(
                113,
                95,
                63,
                (density * strength) as u8,
            ));
        }
    }
    // Fine kozo strands curl in many directions, with light and shadow along each fiber.
    for _ in 0..4200 {
        let x = rand() * n as f32;
        let y = rand() * n as f32;
        let angle = rand() * std::f32::consts::TAU;
        let len = 3. + rand().powi(2) * 38.;
        let bend = (rand() - 0.5) * 5.;
        let alpha = ((8. + rand() * 22.) * strength) as u8;
        for step in 0..len as usize {
            let d = step as f32;
            let curve = (d / len * std::f32::consts::PI).sin() * bend;
            let xx = (x + angle.cos() * d - angle.sin() * curve).rem_euclid(n as f32) as usize;
            let yy = (y + angle.sin() * d + angle.cos() * curve).rem_euclid(n as f32) as usize;
            pixels[yy * n + xx] = Color32::from_rgba_unmultiplied(126, 108, 77, alpha);
            pixels[((yy + 1) % n) * n + xx] = Color32::from_white_alpha(alpha / 2);
        }
    }
    ctx.load_texture(
        "washi-pulp",
        ColorImage::new([n, n], pixels),
        TextureOptions {
            wrap_mode: TextureWrapMode::Repeat,
            ..TextureOptions::LINEAR
        },
    )
}
pub fn paper(p: &Painter, r: Rect, t: &TextureHandle) {
    p.image(
        t.id(),
        r,
        Rect::from_min_max(
            pos2(r.left() / 768., r.top() / 768.),
            pos2(r.right() / 768., r.bottom() / 768.),
        ),
        Color32::WHITE,
    );
}
/// A deferred paper background sized after text layout; only visible edge fibers
/// are generated, even for very long notes.
pub fn sheet_shape(r: Rect, clip: Rect, texture: &TextureHandle, color: Color32) -> Shape {
    let mut shapes = vec![
        Shape::rect_filled(r.translate(vec2(2., 4.)), 1, Color32::from_black_alpha(7)),
        Shape::rect_filled(r, 1, color),
    ];
    let mut mesh = Mesh::with_texture(texture.id());
    mesh.add_rect_with_uv(
        r,
        Rect::from_min_max(
            pos2(r.left() / 768., r.top() / 768.),
            pos2(r.right() / 768., r.bottom() / 768.),
        ),
        Color32::WHITE,
    );
    shapes.push(Shape::mesh(mesh));
    for side in [r.left(), r.right()] {
        let first = (clip.top() - r.top()).floor().max(0.) as usize;
        let last = (clip.bottom() - r.top()).ceil().min(r.height()).max(0.) as usize;
        for y in first..last {
            let yy = r.top() + y as f32;
            let width = 0.7 + 0.65 * (yy * 0.83).sin() + 0.35 * (yy * 2.19).cos();
            shapes.push(Shape::line_segment(
                [pos2(side - width, yy), pos2(side + width, yy)],
                Stroke::new(1., color),
            ));
        }
    }
    shapes.push(Shape::line_segment(
        [
            r.left_top() + vec2(10., 0.),
            r.left_bottom() + vec2(10., 0.),
        ],
        Stroke::new(0.5, LINE),
    ));
    Shape::Vec(shapes)
}
pub fn enso(p: &Painter, c: Pos2, r: f32, color: Color32) {
    for i in 0..90 {
        let t = i as f32 / 90.;
        let a = 0.35 + t * 5.86;
        let b = 0.35 + (i + 1) as f32 / 90. * 5.86;
        let rr = r * (1. + 0.02 * (a * 5.).sin());
        let width = r * (0.045 + 0.10 * (std::f32::consts::PI * t).sin().abs());
        p.line_segment(
            [
                c + vec2(a.cos(), a.sin()) * rr,
                c + vec2(b.cos(), b.sin()) * r,
            ],
            Stroke::new(width, color),
        );
    }
}
/// The original open ink circle with a nib drawn directly on the paper.
/// The slit and breather hole are unpainted, so no background tile is needed.
pub fn brand_mark(p: &Painter, c: Pos2, r: f32, color: Color32) {
    enso(p, c, r, color);
    let mut mesh = Mesh::default();
    let bounds = |y: f32| {
        let outer = if y <= 0.18 {
            0.24 * (y + 0.62) / 0.8
        } else {
            0.24 * (0.58 - y) / 0.4
        };
        let hole = (0.065_f32.powi(2) - (y - 0.18).powi(2)).max(0.).sqrt();
        let slit = if y < 0.18 { 0.018 } else { 0. };
        (outer.max(0.), hole.max(slit).min(outer.max(0.)))
    };
    for i in 0..48 {
        let y0 = -0.62 + i as f32 * 1.2 / 48.;
        let y1 = -0.62 + (i + 1) as f32 * 1.2 / 48.;
        let (outer0, inner0) = bounds(y0);
        let (outer1, inner1) = bounds(y1);
        for side in [-1., 1.] {
            let base = mesh.vertices.len() as u32;
            for (x, y) in [(inner0, y0), (outer0, y0), (outer1, y1), (inner1, y1)] {
                mesh.colored_vertex(c + vec2(x * side, y) * r, ACCENT);
            }
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base, base + 2, base + 3);
        }
    }
    p.add(Shape::mesh(mesh));
}
pub fn landscape(ui: &mut Ui) {
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 106.), Sense::hover());
    let p = ui.painter_at(r);
    let sun = r.min + vec2(r.width() * 0.77, 26.);
    p.circle_filled(sun, 14., Color32::from_rgba_unmultiplied(160, 68, 49, 65));
    for layer in 0..3 {
        let y = 40. + layer as f32 * 14.;
        let color = Color32::from_rgba_unmultiplied(103, 111, 86, 18 + layer * 12);
        let mut mesh = Mesh::default();
        for i in 0..70 {
            let x = i as f32 / 69.;
            let wave =
                (x * 8. + layer as f32 * 1.2).sin() * 15. + (x * 18. + layer as f32).cos() * 5.;
            mesh.colored_vertex(r.min + vec2(x * r.width(), y + wave), color);
            mesh.colored_vertex(pos2(r.left() + x * r.width(), r.bottom()), color);
            if i > 0 {
                let k = (i * 2) as u32;
                mesh.add_triangle(k - 2, k - 1, k);
                mesh.add_triangle(k, k - 1, k + 1);
            }
        }
        p.add(Shape::mesh(mesh));
    }
}

/// Shared brand geometry, also consumed by the demo renderer.
#[derive(serde::Deserialize)]
struct Seal {
    glyph: String,
    width: f32,
    height: f32,
    inset: f32,
    radius: f32,
    stroke: f32,
    font_size: f32,
    color: [u8; 3],
}
/// Small red seal: 静 (quiet), from the original washi design.
pub fn stamp(painter: &Painter, rect: Rect) {
    static SEAL: std::sync::OnceLock<Seal> = std::sync::OnceLock::new();
    let seal = SEAL.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/seal.json")).expect("valid bundled seal")
    });
    let scale = (rect.width() / seal.width).min(rect.height() / seal.height);
    let rect = Rect::from_center_size(rect.center(), vec2(seal.width, seal.height) * scale);
    let color = Color32::from_rgb(seal.color[0], seal.color[1], seal.color[2]);
    painter.rect_stroke(
        rect.shrink(seal.inset * scale),
        seal.radius * scale,
        Stroke::new(seal.stroke * scale, color),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        &seal.glyph,
        FontId::new(seal.font_size * scale, FontFamily::Name("Japanese".into())),
        color,
    );
}

#[cfg(test)]
mod contrast_tests {
    use super::*;
    fn luminance(c: Color32) -> f64 {
        [c.r(), c.g(), c.b()]
            .into_iter()
            .zip([0.2126, 0.7152, 0.0722])
            .map(|(channel, weight)| {
                let v = f64::from(channel) / 255.;
                weight
                    * if v <= 0.04045 {
                        v / 12.92
                    } else {
                        ((v + 0.055) / 1.055).powf(2.4)
                    }
            })
            .sum()
    }
    fn contrast(a: Color32, b: Color32) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
    #[test]
    fn readable_text_tokens_on_supported_flat_surfaces() {
        let mut surfaces = vec![PAPER, PANEL, Color32::from_rgb(223, 217, 193)];
        surfaces.extend(PaperKind::ALL.map(PaperKind::sheet));
        for bg in surfaces {
            for fg in [INK, MUTED, GREEN] {
                assert!(
                    contrast(fg, bg) >= 4.5,
                    "Text {:?} on {:?}: {:.2}",
                    fg,
                    bg,
                    contrast(fg, bg)
                );
            }
        }
        assert!(contrast(ACCENT, PAPER) >= 4.5);
        for paper in PaperKind::ALL {
            assert!(contrast(paper.ink(), paper.sheet()) >= 4.5);
        }
    }
}
