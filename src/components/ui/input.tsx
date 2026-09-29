import {
  forwardRef,
  type InputHTMLAttributes,
  type SelectHTMLAttributes,
} from "react";
import { cn } from "../../lib/utils";
export const Input = forwardRef<
  HTMLInputElement,
  InputHTMLAttributes<HTMLInputElement>
>(({ className, ...props }, ref) => (
  <input ref={ref} className={cn("input", className)} {...props} />
));
Input.displayName = "Input";
export const Select = forwardRef<
  HTMLSelectElement,
  SelectHTMLAttributes<HTMLSelectElement>
>(({ className, ...props }, ref) => (
  <select ref={ref} className={cn("input select", className)} {...props} />
));
Select.displayName = "Select";
