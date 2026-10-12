import { useId, type InputHTMLAttributes } from "react";

export function Switch({
  label,
  id,
  className = "",
  ...props
}: Omit<InputHTMLAttributes<HTMLInputElement>, "type" | "role"> & {
  label: string;
}) {
  const generated = useId();
  return (
    <label htmlFor={id ?? generated} className={`ui-switch ${className}`}>
      <input
        {...props}
        id={id ?? generated}
        type="checkbox"
        role="switch"
        className="peer sr-only"
      />
      <span aria-hidden="true" className="ui-switch-track">
        <span className="ui-switch-thumb" />
      </span>
      <span>{label}</span>
    </label>
  );
}
