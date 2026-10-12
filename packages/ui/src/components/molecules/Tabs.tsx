import type { ReactNode } from "react";
import {
  Tabs as AriaTabs,
  TabList,
  Tab,
  TabPanel,
} from "react-aria-components";
/** Content tabs: arrow-key navigation, linked panels and retained panel state. */
export function Tabs({
  label,
  items,
  value,
  onChange,
}: {
  label: string;
  value?: string;
  onChange?: (value: string) => void;
  items: {
    id: string;
    label: string;
    disabled?: boolean;
    description?: string;
    content: ReactNode;
  }[];
}) {
  return (
    <AriaTabs
      className="ui-content-tabs"
      selectedKey={value}
      onSelectionChange={(key: string | number) => onChange?.(String(key))}
    >
      <TabList aria-label={label} className="ui-content-tab-list">
        {items.map((item) => (
          <Tab
            key={item.id}
            id={item.id}
            isDisabled={item.disabled}
            title={item.description}
            className="ui-content-tab"
          >
            {item.label}
          </Tab>
        ))}
      </TabList>
      {items.map((item) => (
        <TabPanel
          key={item.id}
          id={item.id}
          shouldForceMount
          className="ui-content-panel"
        >
          {item.content}
        </TabPanel>
      ))}
    </AriaTabs>
  );
}
