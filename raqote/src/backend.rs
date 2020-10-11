use crate::{Background, Settings, Viewport};
use fontdue::layout::{GlyphPosition, GlyphRasterConfig};
use fontdue::Metrics;
use iced_graphics::backend;
use iced_graphics::font;
use iced_graphics::Primitive;
use iced_native::mouse;
use iced_native::{Font, HorizontalAlignment, Size, VerticalAlignment};
use log::warn;
use std::{cell::RefCell, collections::HashMap, fmt};

/// A [`raqote`] graphics backend for [`iced`].
///
/// [`raqote`]: https://github.com/jrmuizel/raqote
/// [`iced`]: https://github.com/hecrj/iced
pub struct Backend {
    text_layout: RefCell<fontdue::layout::Layout>,
    glyph_positions: RefCell<Vec<GlyphPosition>>,
    layout_runs: RefCell<HashMap<LayoutRun, Vec<GlyphPosition>>>,
    fonts: RefCell<HashMap<&'static str, fontdue::Font>>,
    fallback_font: fontdue::Font,
    glyph_cache: HashMap<GlyphRasterConfig, (Metrics, Vec<u8>)>,
    default_text_size: u16,
}

impl Backend {
    /// Creates a new [`Backend`].
    ///
    /// [`Backend`]: struct.Backend.html
    pub fn new(settings: Settings) -> Self {
        Self {
            text_layout: RefCell::new(fontdue::layout::Layout::new(
                fontdue::layout::CoordinateSystem::PositiveYDown,
            )),
            glyph_positions: RefCell::new(Vec::new()),
            layout_runs: RefCell::new(HashMap::new()),
            fonts: RefCell::new(HashMap::new()),
            fallback_font: fontdue::Font::from_bytes(
                font::FALLBACK,
                Default::default(),
            )
            .unwrap(),
            glyph_cache: HashMap::new(),
            default_text_size: settings.default_text_size,
        }
    }

    /// Draws the provided primitives in the default framebuffer.
    ///
    /// The text provided as overlay will be rendered on top of the primitives.
    /// This is useful for rendering debug information.
    pub fn draw<T: AsRef<str>>(
        &mut self,
        draw_target: &mut raqote::DrawTarget,
        viewport: &Viewport,
        (primitive, mouse_interaction): &(Primitive, mouse::Interaction),
        overlay_text: &[T],
    ) -> mouse::Interaction {
        let viewport_size = viewport.physical_size();
        let scale_factor = viewport.scale_factor() as f32;

        self.draw_primitive(
            draw_target,
            viewport_size,
            scale_factor,
            primitive,
        );

        for text in overlay_text.iter() {
            self.draw_primitive(
                draw_target,
                viewport_size,
                scale_factor,
                &Primitive::Text {
                    content: text.as_ref().to_string(),
                    bounds: iced_native::Rectangle {
                        x: 0.0,
                        y: 0.0,
                        width: viewport_size.width as f32,
                        height: viewport_size.height as f32,
                    },
                    color: iced_native::Color::from_rgb(1.0, 1.0, 1.0),
                    size: 14.0,
                    font: iced_native::Font::Default,
                    horizontal_alignment: HorizontalAlignment::Left,
                    vertical_alignment: VerticalAlignment::Top,
                },
            );
        }

        *mouse_interaction
    }

    fn draw_primitive(
        &mut self,
        draw_target: &mut raqote::DrawTarget,
        viewport_size: Size<u32>,
        scale_factor: f32,
        primitive: &Primitive,
    ) {
        use raqote::{
            AntialiasMode, BlendMode, DrawOptions, PathBuilder, SolidSource,
            Source,
        };

        match primitive {
            Primitive::None => {}
            Primitive::Group { primitives } => {
                for primitive in primitives {
                    self.draw_primitive(
                        draw_target,
                        viewport_size,
                        scale_factor,
                        primitive,
                    );
                }
            }
            Primitive::Text {
                content,
                bounds,
                color,
                size,
                font,
                horizontal_alignment,
                vertical_alignment,
            } => {
                // Draw the bounds as a filled rectangle.
                // draw_target.fill_rect(
                //     bounds.x,
                //     bounds.y,
                //     bounds.width,
                //     bounds.height,
                //     &Source::Solid(SolidSource::from_unpremultiplied_argb(
                //         255, 127, 127, 127,
                //     )),
                //     &Default::default(),
                // );
                let layout_settings = fontdue::layout::LayoutSettings {
                    x: (bounds.x * scale_factor),
                    y: (bounds.y * scale_factor),
                    max_width: Some(bounds.width * scale_factor),
                    max_height: Some(bounds.height * scale_factor),
                    horizontal_align: match horizontal_alignment {
                        HorizontalAlignment::Left => {
                            fontdue::layout::HorizontalAlign::Left
                        }
                        HorizontalAlignment::Center => {
                            fontdue::layout::HorizontalAlign::Center
                        }
                        HorizontalAlignment::Right => {
                            fontdue::layout::HorizontalAlign::Right
                        }
                    },
                    vertical_align: match vertical_alignment {
                        VerticalAlignment::Top => {
                            fontdue::layout::VerticalAlign::Top
                        }
                        VerticalAlignment::Center => {
                            fontdue::layout::VerticalAlign::Middle
                        }
                        VerticalAlignment::Bottom => {
                            fontdue::layout::VerticalAlign::Bottom
                        }
                    },
                    wrap_style: fontdue::layout::WrapStyle::Word,
                    wrap_hard_breaks: true,
                    include_whitespace: false,
                };
                let mut fonts = self.fonts.borrow_mut();
                let iced_font = font.clone();
                let font = match font {
                    Font::Default => &self.fallback_font,
                    Font::External { name, bytes } => {
                        if fonts.contains_key(name) {
                            fonts.get(name).unwrap()
                        } else {
                            match fontdue::Font::from_bytes(
                                *bytes,
                                Default::default(),
                            ) {
                                Ok(ok) => fonts.entry(name).or_insert(ok),
                                Err(err) => {
                                    warn!(
                                        r#"Using fallback font due to error while loading "{}": "{}""#,
                                        name, err
                                    );
                                    &self.fallback_font
                                }
                            }
                        }
                    }
                };
                let mut glyph_positions = self.glyph_positions.borrow_mut();
                let layout_runs = self.layout_runs.borrow_mut();
                let glyph_positions = layout_runs
                    .get(&LayoutRun {
                        content: content.clone(),
                        size: size.to_ne_bytes(),
                        font: iced_font,
                    })
                    .unwrap_or_else(|| {
                        // NOTE: This really shouldn't ever happen, but it's here just in case.
                        glyph_positions.clear();
                        self.text_layout.borrow_mut().layout_horizontal(
                            &[font],
                            &[&fontdue::layout::TextStyle {
                                text: content.as_ref(),
                                px: *size,
                                font_index: 0,
                            }],
                            &layout_settings,
                            &mut glyph_positions,
                        );
                        &*glyph_positions
                    });

                for glyph_pos in glyph_positions.iter() {
                    let (metrics, coverage) =
                        self.glyph_cache.entry(glyph_pos.key).or_insert_with(
                            || font.rasterize(glyph_pos.key.c, *size),
                        );
                    let mut image_data = Vec::with_capacity(coverage.len());
                    // FIXME: Color space and blending.
                    //        Does `raqote` do its lbending in linear sRGB or does it do it in
                    //        "regular" sRGB?
                    //        There is currently an issue where artifacts appear when white text is
                    //        drawn on a green background.
                    //        There is also an issue where programs build with `iced_raqote` will
                    //        panic due to integer overflow in `sw-composite`.
                    for cov in coverage.iter() {
                        let pixel = (((color.a * *cov as f32).floor() as u32)
                            << 24)
                            | (((color.r * *cov as f32).floor() as u32) << 16)
                            | (((color.g * *cov as f32).floor() as u32) << 8)
                            | ((color.b * *cov as f32).floor() as u32);

                        image_data.push(pixel);
                    }
                    draw_target.draw_image_at(
                        glyph_pos.x + bounds.x,
                        glyph_pos.y + bounds.y,
                        &raqote::Image {
                            width: metrics.width as i32,
                            height: metrics.height as i32,
                            data: &image_data,
                        },
                        &DrawOptions {
                            blend_mode: BlendMode::SrcOver,
                            alpha: 1.0,
                            antialias: AntialiasMode::Gray,
                        },
                    );
                }
            }
            Primitive::Quad {
                bounds,
                background,
                border_radius,
                border_width,
                border_color,
            } => {
                let border_radius = *border_radius as f32;
                let border_width = *border_width as f32;
                let rect_path = |border_radius, x, y, xmax, ymax| {
                    let mut pb = PathBuilder::new();
                    if border_radius == 0.0 {
                        pb.move_to(x, y);
                        pb.line_to(xmax, y);
                        pb.line_to(xmax, ymax);
                        pb.line_to(x, ymax);
                    } else {
                        pb.move_to(x, y + border_radius);
                        pb.quad_to(x, y, x + border_radius, y);
                        pb.line_to(xmax - border_radius, y);
                        pb.quad_to(xmax, y, xmax, y + border_radius);
                        pb.line_to(xmax, ymax - border_radius);
                        pb.quad_to(xmax, ymax, xmax - border_radius, ymax);
                        pb.line_to(x + border_radius, ymax);
                        pb.quad_to(x, ymax, x, ymax - border_radius);
                    }
                    pb.close();
                    pb.finish()
                };
                draw_target.fill(
                    &rect_path(
                        border_radius,
                        bounds.x,
                        bounds.y,
                        bounds.x + bounds.width,
                        bounds.y + bounds.height,
                    ),
                    &Source::Solid(SolidSource::from_unpremultiplied_argb(
                        (border_color.a * 255.0) as u8,
                        (border_color.r * 255.0) as u8,
                        (border_color.g * 255.0) as u8,
                        (border_color.b * 255.0) as u8,
                    )),
                    &DrawOptions::new(),
                );
                let path = rect_path(
                    border_radius,
                    bounds.x + border_width,
                    bounds.y + border_width,
                    bounds.x + bounds.width - border_width,
                    bounds.y + bounds.height - border_width,
                );
                match background {
                    Background::Color(color) => {
                        draw_target.fill(
                            &path,
                            &Source::Solid(
                                SolidSource::from_unpremultiplied_argb(
                                    (color.a * 255.0) as u8,
                                    (color.r * 255.0) as u8,
                                    (color.g * 255.0) as u8,
                                    (color.b * 255.0) as u8,
                                ),
                            ),
                            &DrawOptions {
                                blend_mode: BlendMode::SrcOver,
                                alpha: 1.0,
                                antialias: AntialiasMode::Gray,
                            },
                        );
                    }
                }
            }
            Primitive::Image { handle, bounds } => {
                // TODO: Implement image rendering
            }
            Primitive::Svg { handle, bounds } => {
                // TODO: Implement SVG rendering
            }
            Primitive::Clip {
                bounds,
                offset,
                content,
            } => {
                draw_target.push_clip_rect(raqote::IntRect::new(
                    raqote::IntPoint::new(bounds.x as i32, bounds.y as i32),
                    raqote::IntPoint::new(
                        (bounds.x + bounds.width) as i32,
                        (bounds.y + bounds.height) as i32,
                    ),
                ));
                let prev_transform = draw_target.get_transform().clone();
                draw_target.set_transform(
                    &raqote::Transform::create_translation(
                        bounds.x + offset.x as f32,
                        bounds.y + offset.y as f32,
                    ),
                );
                self.draw_primitive(
                    draw_target,
                    viewport_size,
                    scale_factor,
                    &*content,
                );
                draw_target.set_transform(&prev_transform);
                draw_target.pop_clip();
            }
            Primitive::Translate {
                translation,
                content,
            } => {
                let prev_transform = draw_target.get_transform().clone();
                draw_target.set_transform(
                    &raqote::Transform::create_translation(
                        translation.x,
                        translation.y,
                    ),
                );
                self.draw_primitive(
                    draw_target,
                    viewport_size,
                    scale_factor,
                    &*content,
                );
                draw_target.set_transform(&prev_transform);
            }
            Primitive::Mesh2D { buffers, size } => {
                // TODO: The fact that there's mesh rendering here may be a hint that there's room for an
                //       abstraction that sits between `iced_graphics` and `Widget` implementations. Mesh
                //       rendering makes sense for renderers which utilize triangle-based pipelines, but
                //       such rendering is likely more combersome for software-based rasterization.
            }
            Primitive::Cached { cache } => {
                self.draw_primitive(
                    draw_target,
                    viewport_size,
                    scale_factor,
                    &*cache,
                );
            }
        }
    }
}

impl fmt::Debug for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Backend")
            .field("draw_target", &"DrawTarget { ... }")
            .field("default_text_size", &self.default_text_size)
            .finish()
    }
}

impl iced_graphics::Backend for Backend {
    fn trim_measurements(&mut self) {
        //
    }
}

impl backend::Text for Backend {
    const ICON_FONT: Font = font::ICONS;
    const CHECKMARK_ICON: char = font::CHECKMARK_ICON;
    const ARROW_DOWN_ICON: char = font::ARROW_DOWN_ICON;

    fn default_size(&self) -> u16 {
        self.default_text_size
    }

    fn measure(
        &self,
        contents: &str,
        size: f32,
        font: Font,
        bounds: Size,
    ) -> (f32, f32) {
        let mut fonts = self.fonts.borrow_mut();
        let iced_font = font.clone();
        let font = match font {
            Font::Default => &self.fallback_font,
            Font::External { name, bytes } => {
                if fonts.contains_key(name) {
                    fonts.get(name).unwrap()
                } else {
                    match fontdue::Font::from_bytes(bytes, Default::default()) {
                        Ok(ok) => fonts.entry(name).or_insert(ok),
                        Err(err) => {
                            warn!(
                                r#"Using fallback font due to error while loading "{}": "{}""#,
                                name, err
                            );
                            &self.fallback_font
                        }
                    }
                }
            }
        };

        let layout_settings = fontdue::layout::LayoutSettings {
            x: 0.0,
            y: 0.0,
            max_width: Some(bounds.width),
            max_height: Some(bounds.height),
            horizontal_align: fontdue::layout::HorizontalAlign::Left,
            vertical_align: fontdue::layout::VerticalAlign::Top,
            wrap_style: fontdue::layout::WrapStyle::Word,
            wrap_hard_breaks: true,
            include_whitespace: false,
        };

        let mut glyph_positions = self.glyph_positions.borrow_mut();
        self.text_layout.borrow_mut().layout_horizontal(
            &[font],
            &[&fontdue::layout::TextStyle {
                text: contents,
                px: size,
                font_index: 0,
            }],
            &layout_settings,
            &mut glyph_positions,
        );

        let advance_width = glyph_positions.iter().fold(0.0f32, |acc, pos| {
            acc.max(pos.x + font.metrics(pos.key.c, pos.key.px).advance_width)
        });
        let advance_height = glyph_positions.iter().fold(0.0f32, |acc, pos| {
            acc.max(pos.y + font.metrics(pos.key.c, pos.key.px).advance_height)
        });
        let width = glyph_positions
            .iter()
            .fold(0.0f32, |acc, pos| acc.max(pos.x + pos.width as f32));
        let height = glyph_positions
            .iter()
            .fold(0.0f32, |acc, pos| acc.max(pos.y + pos.height as f32));

        let _ = self.layout_runs.borrow_mut().insert(
            LayoutRun {
                content: contents.to_owned(),
                size: size.to_ne_bytes(),
                font: iced_font,
            },
            glyph_positions.drain(..).collect(),
        );

        (advance_width.ceil().max(width), advance_height.max(height))
    }
}

#[cfg(feature = "image")]
impl backend::Image for Backend {
    fn dimensions(&self, _handle: &iced_native::image::Handle) -> (u32, u32) {
        (50, 50)
    }
}

#[cfg(feature = "svg")]
impl backend::Svg for Backend {
    fn viewport_dimensions(
        &self,
        _handle: &iced_native::svg::Handle,
    ) -> (u32, u32) {
        (50, 50)
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct LayoutRun {
    content: String,
    size: [u8; 4],
    font: Font,
}
