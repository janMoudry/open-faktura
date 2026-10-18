use async_trait::async_trait;
use faktura_application::document_style::*;
use sqlx::{PgPool, Row};
pub struct PgStyle(pub PgPool);
fn err(_: impl std::fmt::Display) -> Error {
    Error::Unavailable
}
pub fn decode_image(value: &str) -> Result<image::DynamicImage> {
    use base64::Engine;
    if value.len() > 140000 {
        return Err(Error::Invalid(vec!["image".into()]));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|_| Error::Invalid(vec!["image".into()]))?;
    if bytes.len() > 100000 {
        return Err(Error::Invalid(vec!["image".into()]));
    }
    let read = || {
        image::io::Reader::new(std::io::Cursor::new(&bytes))
            .with_guessed_format()
            .map_err(|_| Error::Invalid(vec!["image".into()]))
    };
    let reader = read()?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png) | Some(image::ImageFormat::Jpeg)
    ) {
        return Err(Error::Invalid(vec!["image".into()]));
    }
    let (w, h) = reader
        .into_dimensions()
        .map_err(|_| Error::Invalid(vec!["image".into()]))?;
    if w == 0 || h == 0 || w > 1024 || h > 1024 {
        return Err(Error::Invalid(vec!["image_dimensions".into()]));
    }
    let decoded = read()?
        .decode()
        .map_err(|_| Error::Invalid(vec!["image".into()]))?;
    // genpdf/printpdf do not support alpha. Composite onto the white paper,
    // including transparent PNG logos, rather than rendering black rectangles.
    let rgba = decoded.to_rgba8();
    let mut rgb = image::RgbImage::new(w, h);
    for (x, y, pixel) in rgba.enumerate_pixels() {
        let alpha = u32::from(pixel[3]);
        let mut output = [0u8; 3];
        for channel in 0..3 {
            output[channel] =
                ((u32::from(pixel[channel]) * alpha + 255 * (255 - alpha) + 127) / 255) as u8;
        }
        rgb.put_pixel(x, y, image::Rgb(output));
    }
    Ok(image::DynamicImage::ImageRgb8(rgb))
}
#[async_trait]
impl StyleStore for PgStyle {
    async fn get(&self, actor: &str, company: &str) -> Result<Snapshot> {
        let mut tx = self.0.begin().await.map_err(err)?;
        let (_, c) = crate::draft::authorize(&mut tx, actor, company, false).await?;
        let row =
            sqlx::query("SELECT revision,data FROM company_document_style WHERE company_id=$1")
                .bind(c)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?;
        let value = if let Some(r) = row {
            Snapshot {
                revision: r.get("revision"),
                data: serde_json::from_value(r.get("data")).map_err(err)?,
            }
        } else {
            Snapshot {
                revision: 0,
                data: Style::default(),
            }
        };
        tx.commit().await.map_err(err)?;
        Ok(value)
    }
    async fn set(&self, actor: &str, company: &str, command: Command) -> Result<Snapshot> {
        let actor: i64 = actor.parse().map_err(|_| Error::NotFound)?;
        let c: i64 = company.parse().map_err(|_| Error::NotFound)?;
        let mut tx = self.0.begin().await.map_err(err)?;
        let role:Option<String>=sqlx::query_scalar("SELECT m.role FROM company c JOIN membership m ON m.company_id=c.id WHERE c.id=$1 AND m.user_id=$2 AND c.deleted_at IS NULL FOR UPDATE OF c").bind(c).bind(actor).fetch_optional(&mut *tx).await.map_err(err)?;
        if role.as_deref() != Some("owner") {
            return Err(if role.is_none() {
                Error::NotFound
            } else {
                Error::Forbidden
            });
        }
        if uuid::Uuid::parse_str(&command.request_id).is_err()
            || command.revision < 0
            || [&command.data.before_items_cs, &command.data.before_items_en]
                .iter()
                .any(|text| {
                    text.chars().count() > 1000
                        || text
                            .chars()
                            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
                })
            || command.data.footer_cs.chars().count() > 500
            || command.data.footer_en.chars().count() > 500
        {
            return Err(Error::Invalid(vec!["style".into()]));
        }
        for image in [&command.data.logo_base64, &command.data.signature_base64]
            .into_iter()
            .flatten()
        {
            decode_image(image)?;
        }
        let input = serde_json::to_value(&command).map_err(err)?;
        if let Some(row)=sqlx::query("SELECT actor_id,input,result FROM document_style_command WHERE company_id=$1 AND request_id=$2::text::uuid").bind(c).bind(&command.request_id).fetch_optional(&mut *tx).await.map_err(err)?{if row.get::<i64,_>("actor_id")!=actor||row.get::<serde_json::Value,_>("input")!=input{return Err(Error::Conflict)}return serde_json::from_value(row.get("result")).map_err(err)}
        let current: Option<i64> =
            sqlx::query_scalar("SELECT revision FROM company_document_style WHERE company_id=$1")
                .bind(c)
                .fetch_optional(&mut *tx)
                .await
                .map_err(err)?;
        if current.unwrap_or(0) != command.revision {
            return Err(Error::Conflict);
        }
        let revision:i64=sqlx::query_scalar("INSERT INTO company_document_style(company_id,data,updated_by) VALUES($1,$2,$3) ON CONFLICT(company_id) DO UPDATE SET data=excluded.data,revision=company_document_style.revision+1,updated_by=excluded.updated_by,updated_at=now() RETURNING revision").bind(c).bind(serde_json::to_value(&command.data).map_err(err)?).bind(actor).fetch_one(&mut *tx).await.map_err(err)?;
        let result = Snapshot {
            revision,
            data: command.data,
        };
        sqlx::query("INSERT INTO document_style_command(company_id,request_id,actor_id,input,result) VALUES($1,$2::text::uuid,$3,$4,$5)").bind(c).bind(command.request_id).bind(actor).bind(input).bind(serde_json::to_value(&result).map_err(err)?).execute(&mut *tx).await.map_err(err)?;
        tx.commit().await.map_err(err)?;
        Ok(result)
    }
}
