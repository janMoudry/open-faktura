//! Invoice delivery uses the same validation and lifecycle as quotation delivery.
use crate::delivery::{self, Delivery, DeliveryStore, Result, SendQuote};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SendInvoice {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachment_ids: Vec<String>,
    pub request_id: String,
    pub revision: i64,
    pub recipient: String,
    pub subject: String,
    pub body: String,
}
pub async fn send(
    s: &dyn DeliveryStore,
    a: &str,
    c: &str,
    id: &str,
    cmd: SendInvoice,
) -> Result<Delivery> {
    delivery::send(
        s,
        a,
        c,
        id,
        SendQuote {
            attachment_ids: cmd.attachment_ids,
            request_id: cmd.request_id,
            revision: cmd.revision,
            recipient: cmd.recipient,
            subject: cmd.subject,
            body: cmd.body,
            share_id: None,
        },
    )
    .await
}
