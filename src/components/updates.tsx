import { useCallback, useEffect, useRef, useState } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { api, native } from "../lib/api";
import { Button } from "./ui/button";
import { message } from "../lib/utils";

export function Updates() {
  const [update, setUpdate] = useState<Update | null>(null);
  const [status, setStatus] = useState("");
  const [busy, setBusy] = useState(false);
  const [shortcutError, setShortcutError] = useState<string | null>(null);
  const lock = useRef(false);
  const current = useRef<Update | null>(null);
  const mounted = useRef(true);
  const checkUpdates = useCallback(async () => {
    if (!native || lock.current) return;
    lock.current = true;
    setBusy(true);
    try {
      if (!(await api.updaterReady())) {
        if (mounted.current)
          setStatus(
            "Automatic updates will be available in a release configured for signed updates.",
          );
        return;
      }
      const next = await check({ timeout: 15000 });
      if (!mounted.current) {
        await next?.close();
        return;
      }
      await current.current?.close();
      current.current = next;
      setUpdate(next);
      setStatus(
        next ? `Version ${next.version} is available` : "You’re up to date.",
      );
    } catch (e) {
      if (mounted.current)
        setStatus(`Could not check for updates. ${message(e)}`);
    } finally {
      lock.current = false;
      if (mounted.current) setBusy(false);
    }
  }, []);
  useEffect(() => {
    mounted.current = true;
    if (native)
      void api
        .shortcutStatus()
        .then(setShortcutError)
        .catch(() => {});
    const timer = setTimeout(() => void checkUpdates(), 8000);
    const interval = setInterval(
      () => void checkUpdates(),
      24 * 60 * 60 * 1000,
    );
    return () => {
      mounted.current = false;
      clearTimeout(timer);
      clearInterval(interval);
      void current.current?.close();
      current.current = null;
    };
  }, [checkUpdates]);
  return (
    <details className="app-tools" open={update ? true : undefined}>
      <summary>
        {update
          ? `Update available · ${update.version}`
          : "Quick switch & app updates"}
      </summary>
      <p>
        Quick switch from any app: <kbd>⌘ / Ctrl Shift G</kbd>. Escape dismisses
        the picker.
      </p>
      {shortcutError && (
        <p role="alert" className="inline-error">
          Shortcut unavailable: {shortcutError}. Close the app using this
          shortcut, then restart Git Context.
        </p>
      )}
      {status && <p role="status">{status}</p>}
      {update?.body && <pre className="release-notes">{update.body}</pre>}
      <div className="row">
        <Button
          size="sm"
          variant="outline"
          disabled={!native || busy}
          onClick={() => void checkUpdates()}
        >
          Check for updates
        </Button>
        {update && (
          <Button
            size="sm"
            disabled={busy}
            onClick={async () => {
              if (lock.current) return;
              lock.current = true;
              setBusy(true);
              try {
                setStatus("Downloading update…");
                await update.downloadAndInstall((event) => {
                  if (event.event === "Finished")
                    setStatus("Download complete. Installing signed update…");
                });
                setStatus(
                  "Update installed. Quit and reopen Git Context to use the new version.",
                );
                await update.close();
                current.current = null;
                setUpdate(null);
              } catch (e) {
                setStatus(`Update failed. ${message(e)}`);
              } finally {
                lock.current = false;
                setBusy(false);
              }
            }}
          >
            Install update
          </Button>
        )}
      </div>
    </details>
  );
}
