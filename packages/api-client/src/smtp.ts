import type { components } from "./smtp.generated";
import { SalesFailure } from "./sales";
export type SmtpState = components["schemas"]["SmtpState"];
export type SmtpSave = components["schemas"]["SmtpSave"];
export type SmtpTestResult = components["schemas"]["SmtpTestResult"];
async function request(
  base: string,
  method: string,
  suffix: string,
  body?: unknown,
) {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/installation/smtp${suffix}`,
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
    const error = await r.json().catch(() => null);
    throw new SalesFailure(
      r.status,
      error?.code ?? "unavailable",
      error?.fields ?? [],
    );
  }
  return r.json();
}
function state(v: unknown): SmtpState {
  const s = v as SmtpState;
  if (
    !s ||
    !Number.isSafeInteger(s.revision) ||
    !["custom", "disabled", "environment"].includes(s.mode) ||
    !["starttls", "tls", "plain"].includes(s.security) ||
    !Number.isInteger(s.port) ||
    s.port < 1 ||
    s.port > 65535 ||
    [s.host, s.sender, s.username].some((x) => typeof x !== "string") ||
    [s.password_set, s.encryption_ready, s.configured].some(
      (x) => typeof x !== "boolean",
    )
  )
    throw new SalesFailure(502, "invalid_response");
  return s;
}
export async function getSmtp(base: string) {
  return state(await request(base, "GET", ""));
}
export async function saveSmtp(base: string, cmd: SmtpSave) {
  return state(await request(base, "PUT", "", cmd));
}
export async function testSmtp(
  base: string,
  cmd: components["schemas"]["SmtpTest"],
): Promise<SmtpTestResult> {
  const v = await request(base, "POST", "/test", cmd);
  if (
    !v ||
    typeof v.request_id !== "string" ||
    !["sending", "sent", "failed", "uncertain"].includes(v.status) ||
    !(v.error_code === null || typeof v.error_code === "string")
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
