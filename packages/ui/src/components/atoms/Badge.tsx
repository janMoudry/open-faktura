import { Icon } from "./Icon";
import type { ReactNode } from "react";
export type Tone = "neutral" | "success" | "warning" | "danger";
export const toneClasses: Record<Tone, string> = {
  neutral: "bg-canvas text-muted",
  success: "bg-success-soft text-success",
  warning: "bg-warning-soft text-warning",
  danger: "bg-danger-soft text-danger",
};
export function Badge({
  tone = "neutral",
  children,
}: {
  tone?: Tone;
  children: ReactNode;
}) {
  return (
    <span
      className={`inline-flex shrink-0 whitespace-nowrap min-h-6 items-center gap-1.5 rounded-md px-2 py-1 text-xs leading-4 font-medium ${toneClasses[tone]}`}
    >
      {tone !== "neutral" && (
        <Icon
          name={
            tone === "success"
              ? "check"
              : tone === "warning"
                ? "clock"
                : "warning"
          }
          className="h-3 w-3"
        />
      )}
      {children}
    </span>
  );
}
