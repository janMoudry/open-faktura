import type { HTMLAttributes } from "react";

export type HeadingSize = "page" | "section" | "card" | "label";
export function Heading({
  level = 2,
  size = "section",
  className = "",
  ...props
}: HTMLAttributes<HTMLHeadingElement> & {
  level?: 1 | 2 | 3 | 4 | 5 | 6;
  size?: HeadingSize;
}) {
  const Tag = `h${level}` as const;
  return (
    <Tag {...props} className={`ui-heading ui-heading-${size} ${className}`} />
  );
}
