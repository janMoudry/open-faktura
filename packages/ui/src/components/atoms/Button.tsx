import type { ButtonHTMLAttributes } from "react";
import { Icon } from "./Icon";
export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md";
  loading?: boolean;
}
export function Button({
  variant = "primary",
  size = "md",
  loading = false,
  disabled,
  type = "button",
  className = "",
  children,
  ...props
}: ButtonProps) {
  return (
    <button
      {...props}
      type={type}
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      className={`ui-button ui-button-${variant} ui-button-${size} ${className}`}
    >
      {loading && (
        <Icon name="loading" className="h-4 w-4 motion-safe:animate-spin" />
      )}
      {children}
    </button>
  );
}
export function IconButton({
  label,
  icon,
  ...props
}: Omit<ButtonProps, "children" | "aria-label"> & {
  label: string;
  icon: Parameters<typeof Icon>[0]["name"];
}) {
  return (
    <Button
      {...props}
      aria-label={label}
      title={label}
      className={`ui-icon-button ${props.className ?? ""}`}
    >
      {!props.loading && <Icon name={icon} />}
    </Button>
  );
}
