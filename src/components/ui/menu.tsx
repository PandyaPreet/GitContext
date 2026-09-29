import * as Menu from "@radix-ui/react-dropdown-menu";
import { Check } from "lucide-react";
import type { ReactNode } from "react";
export function Dropdown({
  trigger,
  label,
  children,
  align = "end",
}: {
  trigger: ReactNode;
  label: string;
  children: ReactNode;
  align?: "start" | "end";
}) {
  return (
    <Menu.Root>
      <Menu.Trigger asChild>{trigger}</Menu.Trigger>
      <Menu.Portal>
        <Menu.Content
          aria-label={label}
          className="dropdown-content"
          align={align}
          sideOffset={6}
        >
          {children}
        </Menu.Content>
      </Menu.Portal>
    </Menu.Root>
  );
}
export function MenuItem({
  children,
  onSelect,
  disabled,
  selected,
}: {
  children: ReactNode;
  onSelect: () => void;
  disabled?: boolean;
  selected?: boolean;
}) {
  return (
    <Menu.Item className="menu-item" disabled={disabled} onSelect={onSelect}>
      {children}
      {selected && <Check className="menu-check" size={16} />}
    </Menu.Item>
  );
}
export function MenuSeparator() {
  return <Menu.Separator className="menu-separator" />;
}
export function MenuLabel({ children }: { children: ReactNode }) {
  return <Menu.Label className="menu-label">{children}</Menu.Label>;
}
