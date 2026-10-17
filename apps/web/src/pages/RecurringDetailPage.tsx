import { useParams, useSearchParams } from "react-router";
import { useWorkspace } from "../hooks/useCompanyWorkspace";
import { RecurringEditor } from "../components/organisms/RecurringEditor";
export function RecurringDetailPage() {
  const { companyId = "", recurringId = "" } = useParams();
  const { company } = useWorkspace();
  const [params] = useSearchParams();
  return company?.id === companyId ? (
    <RecurringEditor
      key={`${companyId}:${recurringId}`}
      company={company}
      id={recurringId}
      source={params.get("invoice") ?? ""}
    />
  ) : null;
}
