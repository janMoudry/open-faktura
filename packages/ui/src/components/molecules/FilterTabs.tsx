import { useLayoutEffect, useRef, useState } from "react";

export function FilterTabs<T extends string>({
  label,
  value,
  onChange,
  options,
}: {
  label: string;
  value: T;
  onChange: (value: T) => void;
  options: { value: T; label: string; count?: number; disabled?: boolean }[];
}) {
  const root = useRef<HTMLDivElement>(null);
  const [indicator, setIndicator] = useState<{
    x: number;
    y: number;
    width: number;
  } | null>(null);
  const pointer = useRef(false);
  useLayoutEffect(() => {
    const element = root.current;
    if (!element) return;
    const measure = () => {
      const active = element.querySelector<HTMLButtonElement>(
        '[aria-pressed="true"]',
      );
      if (!active) {
        setIndicator(null);
        return;
      }
      const next = {
        x: active.offsetLeft,
        y: active.offsetTop + active.offsetHeight - 2,
        width: active.offsetWidth,
      };
      setIndicator((previous) =>
        previous?.x === next.x &&
        previous.y === next.y &&
        previous.width === next.width
          ? previous
          : next,
      );
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    element
      .querySelectorAll("button")
      .forEach((button) => observer.observe(button));
    return () => observer.disconnect();
  }, [value, options]);
  return (
    <div ref={root} role="group" aria-label={label} className="ui-filter-tabs">
      {options.map((option) => (
        <button
          type="button"
          key={option.value}
          disabled={option.disabled}
          aria-pressed={value === option.value}
          onClick={(event) => {
            pointer.current = event.detail > 0;
            onChange(option.value);
          }}
          className={`ui-filter-tab ${value === option.value ? "font-semibold text-primary" : "text-muted hover:text-foreground"}`}
        >
          {option.label}
          {option.count !== undefined && (
            <span
              className={`rounded px-1.5 py-0.5 text-xs tabular-nums ${value === option.value ? "bg-primary-soft" : "bg-canvas"}`}
            >
              {option.count}
            </span>
          )}
        </button>
      ))}
      {indicator && (
        <span
          aria-hidden="true"
          className="ui-tab-indicator"
          data-animate={pointer.current || undefined}
          style={{
            translate: `${indicator.x}px ${indicator.y}px`,
            width: indicator.width,
          }}
        />
      )}
    </div>
  );
}
