import { AuthFailure } from "@open-faktura/api-client";
import type { AppKey } from "../locales";
export function authMessage(error: unknown): AppKey {
  if (error instanceof AuthFailure) {
    if (error.code === "unauthorized") return "unauthorized";
    if (error.code === "already_configured") return "alreadyConfigured";
    if (error.code === "rate_limited") return "rateLimited";
    if (error.code === "invalid_input") return "invalidInput";
  }
  return "requestFailed";
}
