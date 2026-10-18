use async_trait::async_trait;
use faktura_application::{
    company::{CompanyError as Error, Result},
    draft::DraftStore,
    export::File,
    isdoc::IsdocStore,
};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
pub struct PgIsdoc(pub PgPool);
#[async_trait]
impl IsdocStore for PgIsdoc {
    async fn export(&self, actor: &str, company: &str, id: &str) -> Result<File> {
        // Authorization and snapshot retrieval use the ordinary shared document query.
        let d = crate::draft::PgDraft(self.0.clone())
            .get(actor, company, id)
            .await?;
        if d.number.is_none() {
            return Err(Error::Conflict);
        }
        let document_uuid: uuid::Uuid = sqlx::query_scalar(
            "SELECT request_id FROM invoice_draft WHERE company_id=$1 AND id=$2",
        )
        .bind(company.parse::<i64>().map_err(|_| Error::NotFound)?)
        .bind(id.parse::<i64>().map_err(|_| Error::NotFound)?)
        .fetch_one(&self.0)
        .await
        .map_err(|_| Error::Unavailable)?;
        // Request IDs are unique only within a company. Bind the export identity to both IDs.
        let mut hash = Sha256::new();
        hash.update(b"Open Faktura ISDOC 6.0.2 UUID v8\0");
        hash.update(document_uuid.as_bytes());
        hash.update(company.as_bytes());
        hash.update(b"/");
        hash.update(id.as_bytes());
        let digest = hash.finalize();
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&digest[..16]);
        bytes[6] = (bytes[6] & 0x0f) | 0x80;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        let document_uuid = uuid::Uuid::from_bytes(bytes);
        faktura_application::isdoc::render(&d, &document_uuid.to_string())
    }
}
