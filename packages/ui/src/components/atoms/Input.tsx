import type { InputHTMLAttributes, TextareaHTMLAttributes } from "react";
export function Input({
  className = "",
  ...props
}: InputHTMLAttributes<HTMLInputElement>) {
  return <input {...props} className={`ui-control ${className}`} />;
}
export function Textarea({
  className = "",
  ...props
}: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      rows={3}
      {...props}
      className={`ui-control ui-textarea ${className}`}
    />
  );
}
