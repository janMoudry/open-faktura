import { useLocale } from "../../locales";
import { Alert } from "../molecules/Feedback";
import { Button } from "../atoms/Button";
import { Spinner } from "../atoms/Spinner";
import { Heading } from "../atoms/Heading";
import { useId } from "react";
import {
  Bar,
  CartesianGrid,
  ComposedChart,
  Line,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
  ReferenceLine,
} from "recharts";
import {
  Table,
  TableHeader,
  TableRow,
  TableHead,
  TableCell,
} from "../atoms/Table";

export interface ChartPoint {
  label: string;
  values: Record<string, number | null>;
  /** Exact display labels supplied by the domain adapter, e.g. decimal money. */
  formattedValues?: Record<string, string>;
}

export interface ChartSeries {
  key: string;
  label: string;
}

export interface DataChartProps {
  title: string;
  description: string;
  data: ChartPoint[];
  /** One or two series, distinguished by position or line pattern as well as tone. */
  series: readonly [ChartSeries] | readonly [ChartSeries, ChartSeries];
  variant?: "line" | "bar";
  formatValue?: (value: number) => string;
  loading?: boolean;
  error?: string;
  onRetry?: () => void;
  formatAxis?: (value: number) => string;
}

const colors = ["var(--color-chart-primary)", "var(--color-chart-secondary)"];

export function DataChart({
  title,
  description,
  data,
  series,
  variant = "line",
  loading = false,
  error,
  onRetry,
  formatValue: suppliedFormat,
  formatAxis: suppliedAxis,
}: DataChartProps) {
  const { t, locale } = useLocale();
  const formatValue =
    suppliedFormat ??
    ((value: number) => new Intl.NumberFormat(locale).format(value));
  const formatAxis = suppliedAxis ?? formatValue;
  const id = useId();
  const rows = data.map((point) => ({
    label: point.label,
    values: series.map(({ key }) => {
      const value = point.values[key];
      return typeof value === "number" && Number.isFinite(value) ? value : null;
    }),
  }));
  const hasValues = rows.some((row) => row.values.some((v) => v !== null));
  const display = (index: number, seriesIndex: number) => {
    const value = rows[index]?.values[seriesIndex];
    return value == null
      ? t("noData")
      : (data[index]?.formattedValues?.[series[seriesIndex].key] ??
          formatValue(value));
  };

  return (
    <figure
      aria-labelledby={id}
      aria-busy={loading || undefined}
      className="min-w-0 rounded-panel border border-border bg-surface p-5 sm:p-6"
    >
      <figcaption>
        <Heading level={3} size="card" id={id}>
          {title}
        </Heading>
        <p className="mt-2 text-sm leading-relaxed text-muted">{description}</p>
      </figcaption>
      <ul
        aria-label={t("series", { count: series.length })}
        className="mt-5 flex flex-wrap gap-field text-xs"
      >
        {series.map((item, index) => (
          <li key={item.key} className="flex items-center gap-inline">
            <span
              aria-hidden="true"
              className={
                variant === "bar" ? "h-3 w-3 rounded-sm" : "w-6 border-t-2"
              }
              style={{
                backgroundColor: variant === "bar" ? colors[index] : undefined,
                borderColor: colors[index],
                borderStyle: index ? "dashed" : "solid",
              }}
            />
            {item.label}
          </li>
        ))}
      </ul>
      {loading ? (
        <div className="ui-chart flex items-center justify-center">
          <Spinner label={t("loadingChart")} />
        </div>
      ) : error ? (
        <div className="py-group">
          <Alert tone="danger" role="alert">
            {error}
          </Alert>
          {onRetry && (
            <Button variant="secondary" className="mt-field" onClick={onRetry}>
              {t("retryChart")}
            </Button>
          )}
        </div>
      ) : hasValues ? (
        <div className="ui-chart mt-5">
          <ResponsiveContainer width="100%" height="100%" minWidth={0}>
            <ComposedChart
              data={rows}
              accessibilityLayer
              aria-label={title}
              margin={{ top: 8, right: 8, bottom: 0, left: 0 }}
            >
              <CartesianGrid
                vertical={false}
                stroke="var(--color-border)"
                strokeDasharray="3 4"
              />
              <XAxis
                dataKey="label"
                tickLine={false}
                axisLine={false}
                tick={{ fill: "var(--color-muted)", fontSize: 12 }}
                minTickGap={20}
                dy={8}
              />
              <YAxis
                width={58}
                tickLine={false}
                axisLine={false}
                tick={{ fill: "var(--color-muted)", fontSize: 12 }}
                tickFormatter={formatAxis}
              />
              <ReferenceLine y={0} stroke="var(--color-control-border)" />
              <Tooltip
                isAnimationActive={false}
                cursor={{
                  stroke: "var(--color-control-border)",
                  fill: "var(--color-primary-soft)",
                }}
                content={({
                  active,
                  payload,
                }: {
                  active?: boolean;
                  payload?: ReadonlyArray<{ payload?: unknown }>;
                }) => {
                  if (!active || !payload?.length) return null;
                  const row = payload[0].payload as (typeof rows)[number];
                  const index = rows.indexOf(row);
                  if (index < 0) return null;
                  return (
                    <div className="rounded-lg border border-border bg-surface p-3 text-xs shadow-paper">
                      <p className="mb-2 font-semibold">{row.label}</p>
                      {series.map((item, i) => (
                        <p key={item.key} className="mt-1 tabular-nums">
                          {item.label}: {display(index, i)}
                        </p>
                      ))}
                    </div>
                  );
                }}
              />
              {series.map((item, index) =>
                variant === "bar" ? (
                  <Bar
                    key={item.key}
                    name={item.label}
                    dataKey={`values.${index}`}
                    fill={colors[index]}
                    maxBarSize={32}
                    radius={[3, 3, 0, 0]}
                    isAnimationActive={false}
                  />
                ) : (
                  <Line
                    key={item.key}
                    name={item.label}
                    dataKey={`values.${index}`}
                    type="linear"
                    stroke={colors[index]}
                    strokeWidth={2}
                    strokeDasharray={index ? "6 4" : undefined}
                    dot={{ r: 3, fill: "var(--color-surface)", strokeWidth: 2 }}
                    activeDot={{ r: 5 }}
                    connectNulls={false}
                    isAnimationActive={false}
                  />
                ),
              )}
            </ComposedChart>
          </ResponsiveContainer>
        </div>
      ) : (
        <p className="py-12 text-center text-sm text-muted">
          {t("emptyChart")}
        </p>
      )}
      {!loading && !error && data.length > 0 && (
        <details className="mt-5 border-t border-border pt-4">
          <summary className="cursor-pointer text-sm font-medium">
            {t("showTable")}
          </summary>
          <div className="mt-3 overflow-x-auto">
            <Table aria-label={t("tableLabel", { title })}>
              <TableHeader>
                <TableRow>
                  <TableHead>{t("period")}</TableHead>
                  {series.map((item) => (
                    <TableHead key={item.key} className="text-right">
                      {item.label}
                    </TableHead>
                  ))}
                </TableRow>
              </TableHeader>
              <tbody>
                {rows.map((row, index) => (
                  <TableRow key={index}>
                    <TableCell>{row.label}</TableCell>
                    {series.map((item, i) => (
                      <TableCell
                        key={item.key}
                        className="whitespace-nowrap text-right tabular-nums"
                      >
                        {display(index, i)}
                      </TableCell>
                    ))}
                  </TableRow>
                ))}
              </tbody>
            </Table>
          </div>
        </details>
      )}
    </figure>
  );
}
