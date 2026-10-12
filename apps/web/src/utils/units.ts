import type { AppKey } from "../locales";
export function unitOptions(t: (key: AppKey) => string) {
  return [
    { value: "", label: t("unitNone") },
    { value: "ks", label: t("unitPiece") },
    { value: "hod", label: t("unitHour") },
    { value: "den", label: t("unitDay") },
    { value: "měs.", label: t("unitMonth") },
    { value: "rok", label: t("unitYear") },
    { value: "m", label: "m" },
    { value: "m²", label: "m²" },
    { value: "m³", label: "m³" },
    { value: "km", label: "km" },
    { value: "kg", label: "kg" },
    { value: "g", label: "g" },
    { value: "l", label: "l" },
    { value: "bal.", label: t("unitPack") },
  ];
}
