use color_eyre::eyre::{bail, Result};
use resvg::{tiny_skia, usvg};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("Unhandled error while converting")]
    Unhandled,
}

const SVG_WIDTH: u32 = 350;
const SVG_HEIGHT: u32 = 350;

pub fn svg_to_png(svg_data: &[u8]) -> Result<Vec<u8>> {
    let convert_options = usvg::Options::default();
    let rtree = usvg::Tree::from_data(svg_data, &convert_options)?;

    // Create a pixel map with size 350 x 350
    let mut pixmap = if let Some(result) = tiny_skia::Pixmap::new(SVG_WIDTH, SVG_HEIGHT) {
        result
    } else {
        bail!(ConversionError::Unhandled)
    };

    // Render SVG to pixmap
    resvg::render(
        &rtree,
        usvg::FitTo::Size(SVG_WIDTH, SVG_HEIGHT),
        tiny_skia::Transform::default(),
        pixmap.as_mut(),
    );

    Ok(pixmap.encode_png()?)
}
