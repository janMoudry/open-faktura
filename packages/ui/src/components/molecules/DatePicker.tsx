import { useLocale } from "../../locales";
import {
  type DateSegmentProps,
  DatePicker as AriaDatePicker,
  DateInput,
  DateSegment,
  Group,
  Button,
  Popover,
  Dialog,
  Calendar,
  CalendarGrid,
  CalendarGridHeader,
  CalendarGridBody,
  CalendarHeaderCell,
  CalendarCell,
  Heading,
  I18nProvider,
} from "react-aria-components";
import { parseDate, type CalendarDate } from "@internationalized/date";
import { Icon } from "../atoms/Icon";
import { SelectionLabel, SelectionHelp } from "./Selection";

export interface DatePickerProps {
  label: string;
  value?: string;
  defaultValue?: string;
  onChange?: (value: string) => void;
  min?: string;
  max?: string;
  disabled?: boolean;
  readOnly?: boolean;
  required?: boolean;
  hint?: string;
  error?: string;
  name?: string;
}
const date = (value?: string) => (value ? parseDate(value) : null);
export function DatePicker({
  label,
  value,
  defaultValue,
  onChange,
  min,
  max,
  disabled,
  readOnly,
  required,
  hint,
  error,
  name,
}: DatePickerProps) {
  const { t, locale } = useLocale();
  return (
    <I18nProvider locale={locale}>
      <AriaDatePicker
        className="ui-selection min-w-0"
        value={value === undefined ? undefined : date(value)}
        defaultValue={date(defaultValue)}
        onChange={(next: CalendarDate | null) =>
          onChange?.(next?.toString() ?? "")
        }
        minValue={date(min) ?? undefined}
        maxValue={date(max) ?? undefined}
        isDisabled={disabled}
        isReadOnly={readOnly}
        isRequired={required}
        isInvalid={!!error}
        name={name}
        granularity="day"
        firstDayOfWeek="mon"
        shouldForceLeadingZeros
      >
        <SelectionLabel label={label} required={required} />
        <Group className="ui-compound-control">
          <DateInput className="ui-date-input">
            {(segment: DateSegmentProps["segment"]) => (
              <DateSegment segment={segment} className="ui-date-segment" />
            )}
          </DateInput>
          <Button
            className="ui-control-addon"
            aria-label={t("openCalendar", { label })}
            isDisabled={readOnly}
          >
            <Icon name="calendar" />
          </Button>
        </Group>
        <SelectionHelp hint={hint} error={error} />
        <Popover className="ui-popover ui-calendar-popover" offset={6}>
          <Dialog
            className="outline-none"
            aria-label={t("calendar", { label })}
          >
            <Calendar className="ui-calendar">
              <header className="mb-3 flex items-center justify-between gap-2">
                <Button
                  slot="previous"
                  className="ui-calendar-nav"
                  aria-label={t("previousMonth")}
                >
                  <Icon name="chevronLeft" />
                </Button>
                <Heading className="text-sm font-semibold" />
                <Button
                  slot="next"
                  className="ui-calendar-nav"
                  aria-label={t("nextMonth")}
                >
                  <Icon name="chevronRight" />
                </Button>
              </header>
              <CalendarGrid
                className="w-full border-collapse"
                weekdayStyle="short"
              >
                <CalendarGridHeader>
                  {(day: string) => (
                    <CalendarHeaderCell className="pb-2 text-xs font-medium text-muted">
                      {day}
                    </CalendarHeaderCell>
                  )}
                </CalendarGridHeader>
                <CalendarGridBody>
                  {(day: CalendarDate) => (
                    <CalendarCell date={day} className="ui-calendar-cell" />
                  )}
                </CalendarGridBody>
              </CalendarGrid>
            </Calendar>
          </Dialog>
        </Popover>
      </AriaDatePicker>
    </I18nProvider>
  );
}
