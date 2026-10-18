import { useAppTranslation } from "../../locales";

export function BrandLogo() {
  const { t } = useAppTranslation();
  return (
    <span className="brand-logo">
      <img
        className="brand-logo-light"
        src={`${import.meta.env.BASE_URL}brand/open-faktura.svg`}
        width="512"
        height="192"
        alt={t("brandName")}
      />
      <img
        className="brand-logo-dark"
        src={`${import.meta.env.BASE_URL}brand/open-faktura-inverse.svg`}
        width="512"
        height="192"
        alt=""
        aria-hidden="true"
      />
    </span>
  );
}
