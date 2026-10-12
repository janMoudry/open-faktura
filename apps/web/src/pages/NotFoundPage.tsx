import { Link } from "react-router";
import { Heading } from "@open-faktura/ui";
import { useAppTranslation } from "../locales";
export function NotFoundPage() {
  const { t } = useAppTranslation();
  return (
    <>
      <Heading level={1} size="page">
        {t("notFound")}
      </Heading>
      <Link to="/workspace" className="mt-group inline-block underline">
        {t("backHome")}
      </Link>
    </>
  );
}
