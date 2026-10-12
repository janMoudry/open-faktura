import { useLocale } from "../../locales";
import {
  useId,
  type InputHTMLAttributes,
  type TextareaHTMLAttributes,
  type ReactNode,
} from "react";
import { Input, Textarea } from "../atoms/Input";
import { Icon } from "../atoms/Icon";
import { Focusable, Tooltip, TooltipTrigger } from "react-aria-components";
type Meta = {
  label: string;
  hint?: string;
  error?: string;
  help?: ReactNode;
  horizontal?: boolean;
  trailing?: ReactNode;
};
function useField(
  props: Meta & {
    id?: string;
    "aria-describedby"?: string;
    "aria-invalid"?: InputHTMLAttributes<HTMLInputElement>["aria-invalid"];
  },
) {
  const generated = useId();
  const id = props.id ?? generated;
  return {
    id,
    "aria-invalid": props.error ? true : props["aria-invalid"],
    "aria-describedby":
      [
        props["aria-describedby"],
        props.hint && `${id}-hint`,
        props.error && `${id}-error`,
      ]
        .filter(Boolean)
        .join(" ") || undefined,
  };
}
function Frame({
  id,
  label,
  hint,
  error,
  help,
  horizontal,
  trailing,
  required,
  children,
}: Meta & { id: string; required?: boolean; children: ReactNode }) {
  const { t } = useLocale();
  return (
    <div
      className={
        horizontal
          ? "ui-field-horizontal min-w-0 break-words"
          : "min-w-0 break-words"
      }
    >
      <div className="mb-2 flex items-center gap-1 text-sm font-medium text-foreground">
        <label htmlFor={id}>{label}</label>
        {required && (
          <span aria-hidden="true" className="text-danger">
            *
          </span>
        )}
        {help && (
          <TooltipTrigger delay={200} closeDelay={100}>
            <Focusable>
              <span
                tabIndex={0}
                role="img"
                aria-label={t("help", { label })}
                className="ml-1 inline-flex shrink-0 cursor-help items-center rounded-control text-muted outline-offset-4 hover:text-foreground focus-visible:outline-2 focus-visible:outline-focus"
              >
                <Icon name="info" />
              </span>
            </Focusable>
            <Tooltip
              className="ui-popover w-80 p-field text-sm font-normal leading-relaxed"
              placement="bottom start"
              offset={8}
            >
              {help}
            </Tooltip>
          </TooltipTrigger>
        )}
      </div>
      {horizontal || trailing ? (
        <div className="ui-field-input min-w-0 flex items-start gap-inline">
          {children}
          {trailing}
        </div>
      ) : (
        children
      )}
      {hint && (
        <p
          id={`${id}-hint`}
          className="mt-2 text-xs leading-relaxed text-muted"
        >
          {hint}
        </p>
      )}
      {error && (
        <p
          id={`${id}-error`}
          role="alert"
          className="mt-2 text-xs leading-relaxed text-danger"
        >
          {error}
        </p>
      )}
    </div>
  );
}
export function Field({
  label,
  hint,
  error,
  help,
  horizontal,
  trailing,
  ...props
}: InputHTMLAttributes<HTMLInputElement> & Meta) {
  const a11y = useField({ ...props, label, hint, error });
  return (
    <Frame
      {...{ label, hint, error, help, horizontal, trailing }}
      id={a11y.id}
      required={props.required}
    >
      <Input {...props} {...a11y} />
    </Frame>
  );
}
export function TextareaField({
  label,
  hint,
  error,
  help,
  horizontal,
  trailing,
  ...props
}: TextareaHTMLAttributes<HTMLTextAreaElement> & Meta) {
  const a11y = useField({ ...props, label, hint, error });
  return (
    <Frame
      {...{ label, hint, error, help, horizontal, trailing }}
      id={a11y.id}
      required={props.required}
    >
      <Textarea {...props} {...a11y} />
    </Frame>
  );
}
