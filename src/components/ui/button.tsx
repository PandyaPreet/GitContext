import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "../../lib/utils";
const variants = cva("button", {
  variants: {
    variant: {
      default: "button-primary",
      outline: "button-secondary",
      secondary: "button-secondary",
      ghost: "button-ghost",
      destructive: "button-destructive",
    },
    size: {
      default: "button-default",
      sm: "button-small",
      icon: "button-icon",
    },
  },
  defaultVariants: { variant: "default", size: "default" },
});
export interface ButtonProps
  extends
    React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof variants> {
  asChild?: boolean;
}
export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Component = asChild ? Slot : "button";
    return (
      <Component
        className={cn(variants({ variant, size, className }))}
        ref={ref}
        type="button"
        {...props}
      />
    );
  },
);
Button.displayName = "Button";
