import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Command } from "cmdk";
import { api } from "../lib/api";
import type { AppData } from "../lib/types";
import { message } from "../lib/utils";

export function QuickSwitcher() {
  const [data, setData] = useState<AppData>();
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const lock = useRef(false);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    let disposed = false;
    const refresh = () => {
      setQuery("");
      setError("");
      void api
        .snapshot()
        .then((d) => {
          if (!disposed) {
            setData(d);
            document.documentElement.dataset.theme = d.settings.theme;
          }
        })
        .catch((e) => {
          if (!disposed) setError(message(e));
        });
      input.current?.focus();
    };
    refresh();
    const off = listen("switcher-opened", refresh);
    const changed = listen<AppData>("context-changed", (e) =>
      setData(e.payload),
    );
    const escape = (e: KeyboardEvent) => {
      if (e.key === "Escape") void api.dismissSwitcher();
    };
    window.addEventListener("keydown", escape);
    return () => {
      disposed = true;
      void off.then((f) => f());
      void changed.then((f) => f());
      window.removeEventListener("keydown", escape);
    };
  }, []);
  return (
    <section className="quick-switcher">
      <header>
        <strong>Switch Git profile</strong>
        <button
          aria-label="Close picker"
          onClick={() => void api.dismissSwitcher()}
        >
          Esc
        </button>
      </header>
      <Command loop>
        <Command.Input
          ref={input}
          autoFocus
          value={query}
          onValueChange={setQuery}
          placeholder="Find a profile…"
        />
        <Command.List>
          <Command.Empty>
            No matching profiles. Add one in Git Context.
          </Command.Empty>
          {data?.profiles.map((p) => (
            <Command.Item
              key={p.id}
              value={`${p.name} ${p.gitEmail}`}
              disabled={busy}
              onSelect={() => {
                if (lock.current) return;
                lock.current = true;
                setBusy(true);
                setError("");
                void api
                  .activateProfile(p.id)
                  .then(() => api.dismissSwitcher())
                  .catch((e) => setError(message(e)))
                  .finally(() => {
                    lock.current = false;
                    setBusy(false);
                  });
              }}
            >
              <span className={`profile-dot color-${p.color}`} />
              <span className="grow">
                <strong>{p.name}</strong>
                <small>{p.gitEmail}</small>
              </span>
              {data.singleProfileMode && data.globalProfileId === p.id && (
                <span className="muted">Active</span>
              )}
            </Command.Item>
          ))}
        </Command.List>
      </Command>
      {error && (
        <p className="inline-error" role="alert">
          {error}
        </p>
      )}
      <footer>
        {busy
          ? "Switching safely…"
          : "↑ ↓ Navigate · Enter Switch · Esc Dismiss"}
      </footer>
    </section>
  );
}
