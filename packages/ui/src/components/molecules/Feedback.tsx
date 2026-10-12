import type { ReactNode, HTMLAttributes } from "react";
import { toneClasses, type Tone } from "../atoms/Badge";
import { Icon } from "../atoms/Icon";
export function Alert({
  tone = "neutral",
  children,
  className = "",
  ...props
}: HTMLAttributes<HTMLDivElement> & { tone?: Tone }) {
  return (
    <div
      {...props}
      className={`flex items-start gap-control rounded-lg border border-current/15 p-4 text-sm leading-relaxed ${toneClasses[tone]} ${className}`}
    >
      <Icon
        name={
          tone === "danger" || tone === "warning"
            ? "warning"
            : tone === "success"
              ? "check"
              : "info"
        }
        className="mt-0.5 h-4 w-4"
      />
      <div className="min-w-0 break-words">{children}</div>
    </div>
  );
}
export function EmptyState({
  title,
  description,
  action,
}: {
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <div className="flex flex-col items-center px-5 py-12 text-center">
      <span className="mb-4 rounded-xl bg-primary-soft p-3 text-primary">
        <Icon name="invoice" className="h-6 w-6" />
      </span>
      <p className="font-semibold">{title}</p>
      <p className="mt-2 max-w-sm text-sm leading-relaxed text-muted">
        {description}
      </p>
      {action && <div className="mt-5">{action}</div>}
    </div>
  );
}
