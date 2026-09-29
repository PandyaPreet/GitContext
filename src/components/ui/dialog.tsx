import * as DialogPrimitive from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import { createContext, useContext, type ReactNode } from "react";
export const OperationErrorContext = createContext<string | null>(null);
export function Modal({
  open,
  onOpenChange,
  title,
  description,
  children,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  children: ReactNode;
}) {
  const error = useContext(OperationErrorContext);
  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="dialog-overlay" />
        <DialogPrimitive.Content className="dialog-content">
          <DialogPrimitive.Title className="dialog-title">
            {title}
          </DialogPrimitive.Title>
          <DialogPrimitive.Description className="dialog-description">
            {description}
          </DialogPrimitive.Description>
          <DialogPrimitive.Close
            className="dialog-close"
            aria-label="Close dialog"
          >
            <X size={16} />
          </DialogPrimitive.Close>
          {error && (
            <p role="alert" className="inline-error">
              {error}
            </p>
          )}
          {children}
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
