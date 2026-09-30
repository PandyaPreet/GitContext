import { useCallback, useEffect, useRef, useState } from "react";
import { ShieldCheck, Stethoscope } from "lucide-react";
import { api, native } from "../lib/api";
import type { HealthReport, Profile, Verification } from "../lib/types";
import type { Workspace } from "../hooks/use-workspace";
import { Button } from "./ui/button";
import { message } from "../lib/utils";

export function IdentityHealth({
  workspace: w,
  active,
  activate,
}: {
  workspace: Workspace;
  active?: Profile;
  activate: (p: Profile) => void;
}) {
  const [report, setReport] = useState<HealthReport>();
  const [error, setError] = useState("");
  const [expanded, setExpanded] = useState(false);
  const [checking, setChecking] = useState(false);
  const [verification, setVerification] = useState<{
    result: Verification;
    time: string;
  }>();
  const generation = useRef(0);
  const checkingRef = useRef(false);
  const scan = useCallback(async () => {
    if (!native || checkingRef.current) return;
    checkingRef.current = true;
    setChecking(true);
    const ticket = generation.current;
    try {
      const next = await api.health();
      if (ticket !== generation.current) return;
      setReport(next);
      setError("");
      if (next.checks.some((c) => !c.ok)) {
        setVerification(undefined);
        w.setVerified({});
      }
    } catch (e) {
      if (ticket === generation.current) {
        setError(message(e));
        setVerification(undefined);
        w.setVerified({});
      }
    } finally {
      checkingRef.current = false;
      setChecking(false);
    }
  }, [w.setVerified]);
  useEffect(() => {
    generation.current++;
    setVerification(undefined);
    setReport(undefined);
    void scan();
    const focus = () => {
      void scan();
    };
    window.addEventListener("focus", focus);
    const timer = setInterval(focus, 60000);
    return () => {
      generation.current++;
      window.removeEventListener("focus", focus);
      clearInterval(timer);
    };
  }, [w.data, scan]);
  const issues = report?.checks.filter((c) => !c.ok).length;
  return (
    <section className="identity-health" aria-label="Identity health">
      <div className="health-toolbar">
        <span className="grow">
          <ShieldCheck size={15} />{" "}
          {verification?.result.success
            ? `Verified @${verification.result.authenticatedAs} · ${verification.time}`
            : "Identity not verified"}
        </span>
        <Button
          size="sm"
          variant="ghost"
          disabled={!native || !!w.busy || !active}
          onClick={() =>
            void w.run("Verifying active identity", async () => {
              const ticket = generation.current;
              setVerification(undefined);
              w.setVerified({});
              const result = await api.verifyActive();
              if (ticket !== generation.current) return;
              if (active)
                w.setVerified((current) => ({
                  ...current,
                  [active.id]: result,
                }));
              setVerification({
                result,
                time: new Date().toLocaleTimeString(),
              });
              w.setNotice({
                error: !result.success,
                text:
                  result.message +
                  (result.authenticatedAs
                    ? ` Account: @${result.authenticatedAs}.`
                    : ""),
              });
            })
          }
        >
          Verify active identity
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => {
            setExpanded((v) => !v);
            void scan();
          }}
        >
          <Stethoscope size={14} />
          {checking
            ? "Checking…"
            : error
              ? "Health check unavailable"
              : issues
                ? `${issues} health issue${issues === 1 ? "" : "s"}`
                : report
                  ? "Configuration healthy"
                  : "Health check"}
        </Button>
      </div>
      {expanded && (
        <div className="health-details">
          <p className="muted">
            Checks global defaults. Local repository settings, terminal
            environment overrides, HTTPS credentials and repository access are
            separate.
          </p>
          {error && (
            <p role="alert" className="inline-error">
              {error}
            </p>
          )}
          {report?.checks.map((c) => (
            <div className="health-check" key={c.label}>
              <strong>
                {c.ok ? "✓" : "!"} {c.label}
              </strong>
              <span>{c.detail}</span>
            </div>
          ))}
          <p className="muted">
            For a missing key, restore it or select another profile. For changed
            settings, review Profile details and reapply. Conflicting managed
            edits need manual review or Settings → Configuration history.
          </p>
          <div className="row">
            <Button
              size="sm"
              variant="outline"
              disabled={!native || checking}
              onClick={() => void scan()}
            >
              Check again
            </Button>
            {active && (
              <Button
                size="sm"
                disabled={!!w.busy}
                onClick={() => activate(active)}
              >
                Reapply {active.name}
              </Button>
            )}
          </div>
        </div>
      )}
    </section>
  );
}
