import { PopoverContainer } from "./PopoverContainer";
import { useLocale } from "../../locales";
import { useRef, useState, type ReactNode } from "react";
import * as Primitive from "@radix-ui/react-dialog";
import { IconButton } from "../atoms/Button";
export function Dialog({
  open,
  onClose,
  title,
  description,
  children,
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  description: string;
  children: ReactNode;
}) {
  const { t } = useLocale();
  const [container, setContainer] = useState<HTMLDivElement | null>(null);
  const [animate, setAnimate] = useState(false);
  const returnFocus = useRef<HTMLElement | null>(null);
  return (
    <Primitive.Root
      open={open}
      onOpenChange={(value) => {
        if (!value) onClose();
      }}
    >
      <Primitive.Portal>
        <Primitive.Overlay
          className="ui-dialog-overlay"
          data-animate={animate || undefined}
        />
        <Primitive.Content
          ref={setContainer}
          className="ui-dialog"
          data-animate={animate || undefined}
          onOpenAutoFocus={() => {
            returnFocus.current =
              document.activeElement instanceof HTMLElement
                ? document.activeElement
                : null;
            setAnimate(
              !!returnFocus.current &&
                !returnFocus.current.matches(":focus-visible"),
            );
          }}
          onCloseAutoFocus={(event) => {
            event.preventDefault();
            if (returnFocus.current?.isConnected) returnFocus.current.focus();
          }}
          onPointerDownOutside={(event) => event.preventDefault()}
        >
          <div className="flex items-start justify-between gap-control">
            <Primitive.Title className="ui-heading ui-heading-section min-w-0">
              {title}
            </Primitive.Title>
            <Primitive.Close asChild>
              <IconButton
                icon="close"
                label={t("closeDialog")}
                variant="ghost"
              />
            </Primitive.Close>
          </div>
          <Primitive.Description className="mt-inline mb-group text-sm leading-relaxed text-muted">
            {description}
          </Primitive.Description>
          <PopoverContainer.Provider value={container ?? undefined}>
            {children}
          </PopoverContainer.Provider>
        </Primitive.Content>
      </Primitive.Portal>
    </Primitive.Root>
  );
}
