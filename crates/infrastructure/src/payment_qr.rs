use qrcode::{Color, EcLevel, QrCode};
/// Raster modules without interpolation, with a four-module quiet zone.
pub fn pdf_image(payload: &str) -> Result<genpdf::elements::Image, genpdf::error::Error> {
    let code =
        QrCode::with_error_correction_level(payload.as_bytes(), EcLevel::M).map_err(|_| {
            genpdf::error::Error::new("QR encoding failed", genpdf::error::ErrorKind::InvalidData)
        })?;
    let n = code.width();
    let scale = 8u32;
    let size = (n as u32 + 8) * scale;
    let mut raster = image::GrayImage::from_pixel(size, size, image::Luma([255]));
    for y in 0..n {
        for x in 0..n {
            if code[(x, y)] == Color::Dark {
                for dy in 0..scale {
                    for dx in 0..scale {
                        raster.put_pixel(
                            (x as u32 + 4) * scale + dx,
                            (y as u32 + 4) * scale + dy,
                            image::Luma([0]),
                        );
                    }
                }
            }
        }
    }
    // Fixed 36 mm print size; enough space even at version 6 with quiet zone.
    Ok(
        genpdf::elements::Image::from_dynamic_image(image::DynamicImage::ImageLuma8(raster))?
            .with_scale((
                36.0 / (f64::from(size) * 25.4 / 300.0),
                36.0 / (f64::from(size) * 25.4 / 300.0),
            )),
    )
}
