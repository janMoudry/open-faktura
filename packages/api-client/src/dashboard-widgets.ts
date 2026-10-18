import type { components } from "./dashboard-widgets.generated";
import { SalesFailure } from "./sales";
export type DashboardLayout = components["schemas"]["Layout"];
export type DashboardWidget = components["schemas"]["Widget"];
export async function dashboardWidgets(
  base: string,
  input?: DashboardLayout,
): Promise<DashboardLayout> {
  const r = await fetch(
    `${base.replace(/\/$/, "")}/api/auth/dashboard-widgets`,
    {
      credentials: "include",
      signal: AbortSignal.timeout(15000),
      ...(input
        ? {
            method: "POST",
            headers: {
              "Content-Type": "application/json",
              "X-Faktura-Request": "1",
            },
            body: JSON.stringify(input),
          }
        : {}),
    },
  );
  if (!r.ok) throw new SalesFailure(r.status, "unavailable");
  const v = await r.json();
  if (
    !Number.isSafeInteger(v?.revision) ||
    !Array.isArray(v.widgets) ||
    v.widgets.length > 2 ||
    v.widgets.some(
      (w: DashboardWidget) =>
        !["finance", "tax"].includes(w.kind) ||
        !["compact", "detailed"].includes(w.variant),
    ) ||
    new Set(v.widgets.map((w: DashboardWidget) => w.kind)).size !==
      v.widgets.length
  )
    throw new SalesFailure(502, "invalid_response");
  return v;
}
