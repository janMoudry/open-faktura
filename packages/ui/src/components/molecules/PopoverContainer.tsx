import { createContext, useContext, type ComponentProps } from "react";
import { Popover } from "react-aria-components";
// Keep nested overlays inside the modal's focus and pointer boundary.
export const PopoverContainer = createContext<Element | undefined>(undefined);
export function ContainedPopover(props: ComponentProps<typeof Popover>) {
  const container = useContext(PopoverContainer);
  return <Popover {...props} UNSTABLE_portalContainer={container} />;
}
