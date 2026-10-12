import {
  Button,
  MenuTrigger,
  Popover,
  Menu,
  MenuItem,
} from "react-aria-components";
import { Icon } from "../atoms/Icon";
export function ActionMenu({
  label,
  items,
  disabled = false,
}: {
  label: string;
  disabled?: boolean;
  items: { id: string; label: string; onAction: () => void }[];
}) {
  return (
    <MenuTrigger>
      <Button
        aria-label={label}
        isDisabled={disabled}
        className="ui-button ui-button-secondary ui-button-sm"
      >
        <Icon name="more" />
      </Button>
      <Popover className="ui-popover p-inline" placement="bottom end">
        <Menu aria-label={label} className="outline-none">
          {items.map((item) => (
            <MenuItem
              key={item.id}
              id={item.id}
              onAction={item.onAction}
              className="cursor-pointer rounded-control px-control py-inline text-sm outline-none data-[focused]:bg-primary-soft"
            >
              {item.label}
            </MenuItem>
          ))}
        </Menu>
      </Popover>
    </MenuTrigger>
  );
}
