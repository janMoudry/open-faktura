import { Heading } from "../atoms/Heading";
import type { HTMLAttributes } from "react";
export function Card({
  title,
  titleLevel = 2,
  children,
  className = "",
  ...props
}: HTMLAttributes<HTMLElement> & { title?: string; titleLevel?: 2 | 3 | 4 }) {
  return (
    <section
      {...props}
      className={`min-w-0 rounded-panel border border-border bg-surface p-5 sm:p-6 ${className}`}
    >
      {title && (
        <Heading level={titleLevel} size="card" className="mb-5">
          {title}
        </Heading>
      )}
      {children}
    </section>
  );
}
