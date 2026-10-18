import type { components } from "./attachment.generated";
import { SalesFailure } from "./sales";
export type DocumentAttachment = components["schemas"]["Attachment"];
export type AttachmentKind = "invoice" | "quote" | "received";
async function request(
  base: string,
  c: string,
  k: AttachmentKind,
  d: string,
  suffix: string,
  method = "GET",
  body?: unknown,
) {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/companies/${encodeURIComponent(c)}/attachments/${k}/${encodeURIComponent(d)}${suffix}`,
    {
      method,
      credentials: "include",
      signal: AbortSignal.timeout(60000),
      headers:
        method === "GET"
          ? undefined
          : { "Content-Type": "application/json", "X-Faktura-Request": "1" },
      body: body === undefined ? undefined : JSON.stringify(body),
    },
  );
  if (!r.ok) {
    const v = await r.json().catch(() => null);
    throw new SalesFailure(r.status, v?.code ?? "unavailable", v?.fields ?? []);
  }
  return r;
}
function metadata(v: DocumentAttachment): DocumentAttachment {
  if (
    !v ||
    [v.id, v.filename, v.media_type, v.sha256, v.created_at].some(
      (x) => typeof x !== "string",
    ) ||
    !Number.isSafeInteger(v.size) ||
    v.size < 1
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export async function listAttachments(
  b: string,
  c: string,
  k: AttachmentKind,
  d: string,
): Promise<DocumentAttachment[]> {
  const v = await (await request(b, c, k, d, "")).json();
  if (!Array.isArray(v)) throw new SalesFailure(502, "invalid_response");
  return v.map(metadata);
}
export async function uploadAttachment(
  b: string,
  c: string,
  k: AttachmentKind,
  d: string,
  file: File,
  key: string,
) {
  if (file.size > 2097152 || file.size === 0)
    throw new SalesFailure(422, "invalid", ["file_size"]);
  const bytes = new Uint8Array(await file.arrayBuffer());
  let binary = "";
  for (let i = 0; i < bytes.length; i += 8192)
    binary += String.fromCharCode(...bytes.subarray(i, i + 8192));
  return metadata(
    await (
      await request(b, c, k, d, "", "POST", {
        request_id: key,
        filename: file.name,
        base64: btoa(binary),
      })
    ).json(),
  );
}
export async function removeAttachment(
  b: string,
  c: string,
  k: AttachmentKind,
  d: string,
  id: string,
) {
  await request(b, c, k, d, `/${encodeURIComponent(id)}`, "DELETE");
}
export async function downloadAttachment(
  b: string,
  c: string,
  k: AttachmentKind,
  d: string,
  id: string,
  delivery?: string,
) {
  return (
    await request(
      b,
      c,
      k,
      d,
      delivery
        ? `/deliveries/${encodeURIComponent(delivery)}/${encodeURIComponent(id)}`
        : `/${encodeURIComponent(id)}`,
    )
  ).blob();
}
