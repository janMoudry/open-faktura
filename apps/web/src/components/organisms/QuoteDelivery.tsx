import type { ReactNode } from "react";
import type { SalesDocument } from "@open-faktura/api-client";
import { DocumentDelivery } from "./DocumentDelivery";
export function QuoteDelivery({
  document,
  ...props
}: {
  document: SalesDocument;
  canSend: boolean;
  onChanged: () => void;
  children: ReactNode;
}) {
  return (
    <DocumentDelivery
      key={document.company_id + ":" + document.id}
      quote={document}
      {...props}
    />
  );
}
