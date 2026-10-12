import type { components } from "./auth.generated";
export type AuthUser = components["schemas"]["User"];
export type SetupStatus = components["schemas"]["Setup"];
export class AuthFailure extends Error {
  constructor(
    public status: number,
    public code: string,
  ) {
    super(code);
  }
}
async function request(
  base: string,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
) {
  const response = await fetch(`${base.replace(/\/$/, "")}/api/auth/${path}`, {
    method: body === undefined ? "GET" : "POST",
    credentials: "include",
    signal,
    headers:
      body === undefined
        ? {}
        : { "Content-Type": "application/json", "X-Faktura-Request": "1" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!response.ok) {
    const error = await response.json().catch(() => null);
    throw new AuthFailure(
      response.status,
      typeof error?.code === "string" ? error.code : "invalid_response",
    );
  }
  return response.status === 204 || response.status === 201
    ? null
    : response.json();
}
function user(value: unknown): AuthUser {
  if (
    !value ||
    typeof value !== "object" ||
    !("id" in value) ||
    typeof value.id !== "string" ||
    !("email" in value) ||
    typeof value.email !== "string" ||
    !("name" in value) ||
    typeof value.name !== "string" ||
    !("installation_admin" in value) ||
    typeof value.installation_admin !== "boolean"
  )
    throw new Error("Invalid session response");
  return value as AuthUser;
}
export async function getSetup(
  base: string,
  signal?: AbortSignal,
): Promise<SetupStatus> {
  const value = await request(base, "setup", undefined, signal);
  if (!value || typeof value.setup_required !== "boolean")
    throw new Error("Invalid setup response");
  return value;
}
export async function getSession(
  base: string,
  signal?: AbortSignal,
): Promise<AuthUser | null> {
  try {
    return user(await request(base, "session", undefined, signal));
  } catch (error) {
    if (error instanceof AuthFailure && error.status === 401) return null;
    throw error;
  }
}
export async function login(
  base: string,
  body: components["schemas"]["Credentials"],
  signal?: AbortSignal,
) {
  return user(await request(base, "login", body, signal));
}
export async function bootstrap(
  base: string,
  body: components["schemas"]["Bootstrap"],
  signal?: AbortSignal,
) {
  await request(base, "setup", body, signal);
}
export async function logout(base: string, signal?: AbortSignal) {
  await request(base, "logout", {}, signal);
}

export async function updateAccount(
  base: string,
  body: components["schemas"]["AccountUpdate"],
  signal?: AbortSignal,
) {
  return user(await request(base, "account", body, signal));
}
