import { useState } from "react";
import { useNavigate } from "react-router";
import { Alert, Button, Dialog, Icon } from "@open-faktura/ui";
import { useAppTranslation } from "../../locales";
import { useTrashCommand } from "../../hooks/useDraftTrash";
export function DraftTrashAction({
  company,
  id,
  revision,
  disabled,
}: {
  company: string;
  id: string;
  revision: number;
  disabled?: boolean;
}) {
  const { t } = useAppTranslation();
  const navigate = useNavigate();
  const [request, setRequest] = useState("");
  const action = useTrashCommand();
  return (
    <>
      <Button
        variant="secondary"
        disabled={disabled}
        onClick={() => setRequest(crypto.randomUUID())}
      >
        <Icon name="trash" />
        {t("draftTrashAction")}
      </Button>
      <Dialog
        open={!!request}
        onClose={() => {
          if (!action.busy) setRequest("");
        }}
        title={t("draftTrashConfirm")}
        description={t("draftTrashHelp")}
      >
        {action.error && (
          <Alert tone="danger" role="alert">
            {t("draftTrashError")}
          </Alert>
        )}
        <div className="flex flex-wrap justify-end gap-control">
          <Button
            variant="secondary"
            disabled={action.busy}
            onClick={() => setRequest("")}
          >
            {t("cancelAction")}
          </Button>
          <Button
            loading={action.busy}
            onClick={async () => {
              if (
                await action.execute(company, {
                  request_id: request,
                  id,
                  revision,
                  action: "trash",
                })
              ) {
                setRequest("");
                void navigate(`/companies/${company}/invoices`);
              }
            }}
          >
            {t("draftTrashAction")}
          </Button>
        </div>
      </Dialog>
    </>
  );
}
