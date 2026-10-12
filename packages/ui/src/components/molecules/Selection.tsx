import { ContainedPopover as Popover } from "./PopoverContainer";
import { useLocale } from "../../locales";
import { useState } from "react";
import {
  Select as AriaSelect,
  SelectValue,
  Button,
  Label,
  Text,
  FieldError,
  ListBox,
  ListBoxItem,
  ComboBox,
  Input,
  Group,
  I18nProvider,
} from "react-aria-components";
import { Icon } from "../atoms/Icon";

export interface Option {
  value: string;
  label: string;
  description?: string;
  disabled?: boolean;
}
export interface SelectionProps {
  label?: string;
  "aria-label"?: string;
  id?: string;
  name?: string;
  hint?: string;
  error?: string;
  disabled?: boolean;
  required?: boolean;
  placeholder?: string;
  options: readonly Option[];
  className?: string;
}
export interface SelectProps extends SelectionProps {
  onOpenChange?: (open: boolean) => void;
  value?: string;
  defaultValue?: string;
  onChange?: (value: string) => void;
}

export function SelectionLabel({
  label,
  required,
}: Pick<SelectionProps, "label" | "required">) {
  return label ? (
    <div className="ui-selection-label mb-2 flex items-center gap-1 text-sm font-medium">
      <Label>{label}</Label>
      {required && (
        <span aria-hidden="true" className="text-danger">
          *
        </span>
      )}
    </div>
  ) : null;
}
export function SelectionHelp({
  hint,
  error,
}: Pick<SelectionProps, "hint" | "error">) {
  const { t } = useLocale();
  return (
    <>
      {hint && (
        <Text
          slot="description"
          className="mt-2 block text-xs leading-relaxed text-muted"
        >
          {hint}
        </Text>
      )}
      <FieldError className="mt-2 block text-xs text-danger">
        {({ validationErrors }: { validationErrors: string[] }) => (
          <span role="alert">
            {error || (validationErrors.length ? t("invalidValue") : "")}
          </span>
        )}
      </FieldError>
    </>
  );
}
function renderOption(option: Option) {
  return (
    <ListBoxItem
      id={option.value}
      textValue={option.label}
      isDisabled={option.disabled}
      className="ui-option"
    >
      {({ isSelected }: { isSelected: boolean }) => (
        <>
          <div className="min-w-0 flex-1 break-words">
            <Text slot="label" className="block">
              {option.label}
            </Text>
            {option.description && (
              <Text
                slot="description"
                className="mt-1 block text-xs text-muted"
              >
                {option.description}
              </Text>
            )}
          </div>
          <span className="w-4 shrink-0">
            {isSelected && <Icon name="check" />}
          </span>
        </>
      )}
    </ListBoxItem>
  );
}
export function Select({
  label,
  hint,
  error,
  disabled,
  required,
  options,
  value,
  defaultValue,
  onChange,
  className = "",
  placeholder,
  ...props
}: SelectProps) {
  const { t, locale } = useLocale();
  return (
    <I18nProvider locale={locale}>
      <AriaSelect
        {...props}
        placeholder={placeholder ?? t("selectPlaceholder")}
        className={`ui-selection min-w-0 ${className}`}
        selectedKey={value}
        defaultSelectedKey={defaultValue}
        onSelectionChange={(key: string | number | null) =>
          onChange?.(key == null ? "" : String(key))
        }
        isDisabled={disabled}
        isRequired={required}
        isInvalid={!!error}
      >
        <SelectionLabel label={label} required={required} />
        <Button className="ui-control ui-select-trigger">
          <SelectValue className="min-w-0 flex-1 truncate" />
          <Icon name="chevronDown" />
        </Button>
        <SelectionHelp hint={hint} error={error} />
        <Popover className="ui-popover ui-select-popover" offset={6}>
          <ListBox
            className="ui-options"
            items={options}
            renderEmptyState={() => (
              <p className="p-3 text-sm text-muted">{t("noOptions")}</p>
            )}
          >
            {renderOption}
          </ListBox>
        </Popover>
      </AriaSelect>
    </I18nProvider>
  );
}

export interface AutocompleteProps extends SelectionProps {
  value?: string;
  defaultValue?: string;
  onChange?: (value: string) => void;
  onSelect?: (value: string) => void;
  loading?: boolean;
  onFocus?: () => void;
}
const normalize = (text: string) =>
  text
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLocaleLowerCase("cs");
export function Autocomplete({
  label,
  hint,
  error,
  disabled,
  required,
  options,
  value,
  defaultValue = "",
  onChange,
  onSelect,
  loading,
  className = "",
  placeholder,
  ...props
}: AutocompleteProps) {
  const { t, locale } = useLocale();
  const [internal, setInternal] = useState(defaultValue);
  const input = value ?? internal;
  const filtered = loading
    ? []
    : options.filter((option) =>
        normalize(option.label + " " + (option.description ?? "")).includes(
          normalize(input),
        ),
      );
  return (
    <I18nProvider locale={locale}>
      <ComboBox
        {...props}
        className={`ui-selection min-w-0 ${className}`}
        inputValue={input}
        onInputChange={(text: string) => {
          setInternal(text);
          onChange?.(text);
        }}
        onSelectionChange={(key: string | number | null) => {
          if (key != null) onSelect?.(String(key));
        }}
        items={filtered}
        allowsCustomValue
        allowsEmptyCollection
        menuTrigger="focus"
        isDisabled={disabled}
        isRequired={required}
        isInvalid={!!error}
      >
        <SelectionLabel label={label} required={required} />
        <Group className="ui-compound-control">
          <Input
            className="ui-combo-input"
            placeholder={placeholder ?? t("selectPlaceholder")}
          />
          <Button
            className="ui-control-addon"
            aria-label={t("showSuggestions")}
          >
            <Icon
              name={loading ? "loading" : "chevronDown"}
              className={
                loading ? "h-4 w-4 motion-safe:animate-spin" : "h-4 w-4"
              }
            />
          </Button>
        </Group>
        <SelectionHelp hint={hint} error={error} />
        <Popover className="ui-popover ui-select-popover" offset={6}>
          <ListBox
            className="ui-options"
            items={filtered}
            renderEmptyState={() => (
              <p className="p-3 text-sm text-muted">
                {loading ? t("loadingSuggestions") : t("noSuggestions")}
              </p>
            )}
          >
            {renderOption}
          </ListBox>
        </Popover>
        {loading && (
          <span role="status" className="sr-only">
            {t("suggestionsStatus")}
          </span>
        )}
      </ComboBox>
    </I18nProvider>
  );
}
