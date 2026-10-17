import { useParams } from "react-router";
import { CreditNoteEditor } from "../components/organisms/CreditNoteEditor";
export function CreditNotePage() {
  const { companyId = "", creditId = "" } = useParams();
  return (
    <CreditNoteEditor
      key={`${companyId}:${creditId}`}
      companyId={companyId}
      id={creditId}
    />
  );
}
