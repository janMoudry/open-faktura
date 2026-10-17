import type { components } from "./preferences.generated";
import { SalesFailure } from "./sales";
export type UserPreferences = components["schemas"]["Preferences"];
async function request(
  base: string,
  data?: UserPreferences,
  signal?: AbortSignal,
): Promise<UserPreferences> {
  const r = await fetch(base.replace(/\/$/, "") + "/api/auth/preferences", {
    method: data ? "POST" : "GET",
    credentials: "include",
    headers: data
      ? { "Content-Type": "application/json", "X-Faktura-Request": "1" }
      : undefined,
    body: data ? JSON.stringify(data) : undefined,
    signal: signal ?? AbortSignal.timeout(15000),
  });
  if (!r.ok) throw new SalesFailure(r.status, "preferences");
  const v = await r.json();
  if (
    !Number.isSafeInteger(v?.revision) ||
    !["system", "light", "dark"].includes(v.theme)
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
export const getUserPreferences = (b: string, s?: AbortSignal) =>
  request(b, undefined, s);
export const setUserPreferences = (b: string, v: UserPreferences) =>
  request(b, v);
