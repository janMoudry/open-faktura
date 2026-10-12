import { Field } from "@open-faktura/ui";
import type { PartySnapshot } from "@open-faktura/api-client";
import { useAppTranslation } from "../../locales";
export function InvoicePartyFields({
  value,
  onChange,
  disabled,
  showRegistration = true,
}: {
  value: PartySnapshot;
  onChange: (v: PartySnapshot) => void;
  disabled: boolean;
  showRegistration?: boolean;
}) {
  const { t } = useAppTranslation();
  return (
    <div className="grid gap-field sm:grid-cols-2">
      {(
        [
          ["name", "contactName"],
          ["registration_number", "registrationNumber"],
          ["vat_number", "vatNumber"],
          ["email", "email"],
          ["phone", "phone"],
        ] as const
      )
        .filter(([key]) => showRegistration || key !== "registration_number")
        .map(([key, label]) => (
          <Field
            key={key}
            label={t(label)}
            value={value[key]}
            readOnly={disabled}
            onChange={(e) => onChange({ ...value, [key]: e.target.value })}
          />
        ))}
      {(
        [
          ["street", "street"],
          ["city", "city"],
          ["postal_code", "postalCode"],
          ["country", "country"],
        ] as const
      ).map(([key, label]) => (
        <Field
          key={key}
          label={t(label)}
          value={value.address[key]}
          readOnly={disabled}
          onChange={(e) =>
            onChange({
              ...value,
              address: { ...value.address, [key]: e.target.value },
            })
          }
        />
      ))}
    </div>
  );
}
