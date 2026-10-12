import type { ReactNode } from "react";
import { ProductSidebar } from "../organisms/ProductSidebar";
export function ProductWorkspace({
  children,
  selected,
  canCreate,
}: {
  children: ReactNode;
  selected?: string;
  canCreate?: boolean;
}) {
  return (
    <div className="workspace-shell">
      <ProductSidebar selected={selected} canCreate={canCreate} />
      <div className="min-w-0 lg:col-start-2">
        <div className="workspace-content">{children}</div>
      </div>
    </div>
  );
}
