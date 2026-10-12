import type { InputHTMLAttributes } from "react";
export function Radio({
  label,
  className = "",
  ...props
}: Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & { label: string }) {
  return (
    <label
      className={`flex min-h-11 items-center gap-3 text-sm has-disabled:cursor-not-allowed has-disabled:text-muted ${className}`}
    >
      <input
        {...props}
        type="radio"
        className="ui-native-choice h-4 w-4 shrink-0 accent-primary"
      />
      {label}
    </label>
  );
}
