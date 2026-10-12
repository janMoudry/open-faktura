import { useLayoutEffect, useRef, type InputHTMLAttributes } from "react";
export function Checkbox({
  label,
  indeterminate = false,
  className = "",
  ...props
}: Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & {
  label: string;
  indeterminate?: boolean;
}) {
  const input = useRef<HTMLInputElement>(null);
  useLayoutEffect(() => {
    if (input.current) input.current.indeterminate = indeterminate;
  }, [indeterminate]);
  return (
    <label
      className={`flex min-h-11 items-center gap-3 text-sm has-disabled:cursor-not-allowed has-disabled:text-muted ${className}`}
    >
      <input
        {...props}
        ref={input}
        aria-checked={indeterminate ? "mixed" : props["aria-checked"]}
        type="checkbox"
        className="ui-native-choice h-4 w-4 shrink-0 accent-primary disabled:opacity-50"
      />
      {label}
    </label>
  );
}
