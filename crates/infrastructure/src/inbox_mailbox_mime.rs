use base64::Engine;
use faktura_application::{
    company::{CompanyError as Error, Result},
    inbox::{MailIntake, Upload},
};
use mailparse::MailHeaderMap;
fn bad() -> Error {
    Error::Invalid(vec!["mailbox_message".into()])
}
pub(super) fn parse(bytes: &[u8], request_id: String, fallback: &str) -> Result<MailIntake> {
    if bytes.is_empty() || bytes.len() > 10 * 1024 * 1024 {
        return Err(bad());
    }
    // Bound MIME recursion before mailparse builds a recursive tree. Header names
    // are ASCII case-insensitive; folded continuations cannot create a new header.
    if bytes
        .split(|b| *b == b'\n')
        .filter(|line| {
            line.iter()
                .copied()
                .skip_while(u8::is_ascii_whitespace)
                .take(13)
                .map(|b| b.to_ascii_lowercase())
                .collect::<Vec<_>>()
                .starts_with(b"content-type:")
        })
        .count()
        > 32
    {
        return Err(bad());
    }
    let parsed = mailparse::parse_mail(bytes).map_err(|_| bad())?;
    fn clean(value: String) -> String {
        value
            .chars()
            .map(|c| {
                if c == '\r' || c == '\n' || c == '\t' {
                    ' '
                } else {
                    c
                }
            })
            .collect::<String>()
            .trim()
            .into()
    }
    let message_id = parsed
        .headers
        .get_first_value("Message-ID")
        .map(clean)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| fallback.into());
    let sender = clean(parsed.headers.get_first_value("From").unwrap_or_default());
    let subject = clean(
        parsed
            .headers
            .get_first_value("Subject")
            .unwrap_or_default(),
    );
    let mut attachments = Vec::new();
    let mut total = 0;
    fn walk(
        part: &mailparse::ParsedMail<'_>,
        depth: usize,
        parts: &mut usize,
        out: &mut Vec<Upload>,
        total: &mut usize,
    ) -> Result<()> {
        *parts += 1;
        if depth > 32 || *parts > 64 {
            return Err(bad());
        }
        if !part.subparts.is_empty() {
            for child in &part.subparts {
                walk(child, depth + 1, parts, out, total)?;
            }
            return Ok(());
        }
        let disposition = part.get_content_disposition();
        if disposition.disposition == mailparse::DispositionType::Inline {
            return Ok(());
        }
        let filename = disposition
            .params
            .get("filename")
            .or_else(|| part.ctype.params.get("name"));
        let Some(filename) = filename else {
            return Ok(());
        };
        let ext = filename
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(ext.as_str(), "pdf" | "png" | "jpg" | "jpeg" | "isdoc") {
            return Ok(());
        }
        let bytes = part.get_body_raw().map_err(|_| bad())?;
        *total += bytes.len();
        if out.len() >= 10 || bytes.is_empty() || bytes.len() > 2097152 || *total > 10 * 1024 * 1024
        {
            return Err(bad());
        }
        out.push(Upload {
            request_id: uuid::Uuid::new_v4().to_string(),
            filename: filename.clone(),
            base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        });
        Ok(())
    }
    walk(&parsed, 0, &mut 0, &mut attachments, &mut total)?;
    if attachments.is_empty() {
        return Err(Error::Invalid(vec!["no_supported_attachments".into()]));
    }
    Ok(MailIntake {
        request_id,
        message_id,
        sender,
        subject,
        attachments,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decode_encoded_headers_attachment_and_ignore_inline() {
        let mail=b"From: sender@example.test\r\nSubject: =?utf-8?B?VGVzdA==?=\r\nMessage-ID: <synthetic@example.test>\r\nContent-Type: multipart/mixed; boundary=bound\r\n\r\n--bound\r\nContent-Type: application/pdf\r\nContent-Disposition: attachment; filename=original.pdf\r\nContent-Transfer-Encoding: base64\r\n\r\nJVBERi0xLjQK\r\n--bound\r\nContent-Type: image/png\r\nContent-Disposition: inline; filename=tracker.png\r\n\r\nignored\r\n--bound--\r\n";
        let result = parse(mail, uuid::Uuid::new_v4().to_string(), "fallback").unwrap();
        assert_eq!(result.subject, "Test");
        assert_eq!(result.attachments.len(), 1);
        assert_eq!(result.attachments[0].filename, "original.pdf");
        assert_eq!(result.message_id, "<synthetic@example.test>");
    }
    #[test]
    fn bound_nested_mime_before_recursive_parse() {
        let nested = format!(
            "{}\r\n",
            "Content-Type: multipart/mixed; boundary=x\r\n".repeat(33)
        );
        assert!(parse(nested.as_bytes(), uuid::Uuid::new_v4().to_string(), "x").is_err());
    }
}
