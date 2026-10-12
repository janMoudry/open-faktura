import { getHealth } from "@open-faktura/api-client";
export const apiUrl = import.meta.env.VITE_API_URL ?? "";
export async function loadStatus(signal: AbortSignal) {
  const [api, database] = await Promise.all([
    getHealth(apiUrl, "health", signal),
    getHealth(apiUrl, "ready", signal),
  ]);
  return { api: api.status === "ok", database: database.status === "ready" };
}
