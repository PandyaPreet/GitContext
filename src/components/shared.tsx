import {
  Github,
  Gitlab,
  CircleHelp,
  Loader2,
  AlertCircle,
  Check,
  Minus,
} from "lucide-react";
import type { ReactNode } from "react";
import type { GitProvider, Verification } from "../lib/types";
import { providerLabel } from "../lib/providers";
import { Button, type ButtonProps } from "./ui/button";
import { Tooltip } from "./ui/tooltip";
export function ProviderMark({
  provider,
  host,
  iconOnly = false,
}: {
  provider?: GitProvider | null;
  host?: string;
  iconOnly?: boolean;
}) {
  const Icon =
    provider === "github"
      ? Github
      : provider === "gitlab"
        ? Gitlab
        : CircleHelp;
  return (
    <span className="provider-mark" title={host}>
      <Icon
        size={14}
        className={provider === "gitlab" ? "provider-gitlab" : ""}
      />
      {!iconOnly && <span>{providerLabel(provider)}</span>}
    </span>
  );
}
export function IconButton({
  label,
  children,
  ...props
}: ButtonProps & { label: string }) {
  return (
    <Tooltip label={label}>
      <Button aria-label={label} variant="ghost" size="icon" {...props}>
        {children}
      </Button>
    </Tooltip>
  );
}
export function EmptyState({
  title,
  description,
  action,
}: {
  title: string;
  description: string;
  action?: ReactNode;
}) {
  return (
    <div className="empty-state">
      <h3>{title}</h3>
      <p>{description}</p>
      {action}
    </div>
  );
}
export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
}) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      {children}
      {hint && <span className="field-hint">{hint}</span>}
    </label>
  );
}
export function ConnectionState({
  value,
  busy,
}: {
  value?: Verification;
  busy?: boolean;
}) {
  return (
    <span
      className={`connection-state ${value?.success ? "success" : value ? "warning" : "muted"}`}
    >
      {busy ? (
        <Loader2 className="spin" size={13} />
      ) : value?.success ? (
        <Check size={13} />
      ) : value ? (
        <AlertCircle size={13} />
      ) : (
        <Minus size={13} />
      )}
      <span>
        {busy
          ? "Testing…"
          : value?.success
            ? "SSH verified"
            : value
              ? "Needs attention"
              : "Not tested"}
      </span>
    </span>
  );
}
export function DataRow({
  label,
  children,
  mono = false,
}: {
  label: string;
  children: ReactNode;
  mono?: boolean;
}) {
  return (
    <div className="data-row">
      <dt>{label}</dt>
      <dd className={mono ? "mono" : ""}>
        {children || <span className="muted">Not set</span>}
      </dd>
    </div>
  );
}
