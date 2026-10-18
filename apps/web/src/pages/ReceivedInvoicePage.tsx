import { useParams } from "react-router";
import { ReceivedInvoiceEditor } from "../components/organisms/ReceivedInvoiceEditor";
export function ReceivedInvoicePage({
  kind = "invoice",
}: {
  kind?: "invoice" | "receipt";
}) {
  const { companyId = "", receivedId = "new" } = useParams();
  return (
    <ReceivedInvoiceEditor
      kind={kind}
      key={kind + `${companyId}:${receivedId}`}
      companyId={companyId}
      id={receivedId}
    />
  );
}
