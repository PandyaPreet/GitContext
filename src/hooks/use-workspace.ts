import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { isEnabled, enable, disable } from "@tauri-apps/plugin-autostart";
import { api, emptyData, native } from "../lib/api";
import { message } from "../lib/utils";
import type {
  AppData,
  Detection,
  Profile,
  Settings,
  SshKey,
  Verification,
} from "../lib/types";

export function useWorkspace() {
  const [data, setData] = useState<AppData>(emptyData);
  const [detection, setDetection] = useState<Detection | null>(null);
  const [verified, setVerified] = useState<Record<string, Verification>>({});
  const [keys, setKeys] = useState<SshKey[]>([]);
  const [transactions, setTransactions] = useState<[string, string][]>([]);
  const [autostart, setAutostart] = useState(false);
  const [loading, setLoading] = useState(native);
  const [busy, setBusy] = useState("");
  const busyRef = useRef(false);
  const [notice, setNotice] = useState<{ error: boolean; text: string } | null>(
    null,
  );
  const [startupSelector, setStartupSelector] = useState(false);
  // Legacy repository metadata is retained solely for safe migration/rollback.
  // The simplified app does not scan or present repositories.
  const refreshStatuses = useCallback(async (_next: AppData) => {}, []);
  const refresh = useCallback(async () => {
    const next = await api.snapshot();
    setData(next);
    await refreshStatuses(next);
    setTransactions(await api.transactions());
  }, [refreshStatuses]);
  const run = useCallback(async (label: string, fn: () => Promise<void>) => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(label);
    setNotice(null);
    try {
      await fn();
    } catch (error) {
      setNotice({ error: true, text: message(error) });
    } finally {
      busyRef.current = false;
      setBusy("");
    }
  }, []);
  useEffect(() => {
    if (!native) return;
    let cancelled = false;
    // Render saved context immediately; optional environment probes must not
    // hold the workspace behind a slow SSH agent or CLI lookup.
    api
      .snapshot()
      .then((next) => {
        if (cancelled) return;
        setData(next);
        setStartupSelector(next.settings.startupView === "selector");
        setLoading(false);
        void refreshStatuses(next);
      })
      .catch((error) => {
        if (!cancelled) {
          setNotice({ error: true, text: message(error) });
          setLoading(false);
        }
      });
    Promise.allSettled([api.detect(), isEnabled(), api.transactions()]).then(
      (results) => {
        if (cancelled) return;
        const [detected, auto, tx] = results;
        if (detected.status === "fulfilled") setDetection(detected.value);
        else
          setNotice({
            error: true,
            text: `Environment detection failed. ${message(detected.reason)}`,
          });
        if (auto.status === "fulfilled") setAutostart(auto.value);
        if (tx.status === "fulfilled") setTransactions(tx.value);
      },
    );
    const changed = listen<AppData>("context-changed", (e) => {
      if (!cancelled) setData(e.payload);
    });
    const errors = listen<string>("context-error", (e) => {
      if (!cancelled) setNotice({ error: true, text: e.payload });
    });
    return () => {
      cancelled = true;
      void changed.then((off) => off());
      void errors.then((off) => off());
    };
  }, [refreshStatuses]);
  useEffect(() => {
    if (!native) return;
    let cancelled = false;
    const paths = [
      ...new Set([
        ...(detection?.sshKeys.map((k) => k.publicPath) ?? []),
        ...data.profiles.map((p) => p.publicKeyPath),
        ...(data.importedKeyPaths ?? []),
      ]),
    ].filter(
      (path) =>
        !(data.hiddenKeyPaths ?? []).includes(path) ||
        data.profiles.some((p) => p.publicKeyPath === path),
    );
    Promise.allSettled(
      paths.map(
        (path) =>
          detection?.sshKeys.find((k) => k.publicPath === path) ??
          api.importKey(path),
      ),
    ).then((results) => {
      if (!cancelled)
        setKeys(
          results.flatMap((r) => (r.status === "fulfilled" ? [r.value] : [])),
        );
    });
    return () => {
      cancelled = true;
    };
  }, [data.profiles, data.importedKeyPaths, data.hiddenKeyPaths, detection]);
  useEffect(() => {
    document.documentElement.dataset.theme = data.settings.theme;
  }, [data.settings.theme]);
  useEffect(() => {
    if (notice && !notice.error) {
      const timer = setTimeout(() => setNotice(null), 4500);
      return () => clearTimeout(timer);
    }
  }, [notice]);
  const testProfile = (profile: Profile) =>
    run(`Testing ${profile.name}`, async () => {
      const result = await api.verify(profile.id);
      setVerified((current) => ({ ...current, [profile.id]: result }));
      setNotice({
        error: !result.success,
        text: `${result.message}${result.authenticatedAs ? ` Account: @${result.authenticatedAs}.` : ""}`,
      });
    });
  const updateSettings = (settings: Settings) =>
    run("Saving settings", async () => {
      if (native) setData(await api.settings(settings));
      else setData((current) => ({ ...current, settings }));
    });
  const setLaunchAtLogin = (checked: boolean) =>
    run("Updating launch at login", async () => {
      if (checked) await enable();
      else await disable();
      setAutostart(await isEnabled());
    });
  const scan = () =>
    run("Refreshing", async () => {
      setDetection(await api.detect());
      await refresh();
    });
  return {
    data,
    setData,
    detection,
    verified,
    setVerified,
    keys,
    setKeys,
    transactions,
    autostart,
    loading,
    busy,
    notice,
    setNotice,
    startupSelector,
    setStartupSelector,
    refresh,
    refreshStatuses,
    run,
    testProfile,
    updateSettings,
    setLaunchAtLogin,
    scan,
  };
}
export type Workspace = ReturnType<typeof useWorkspace>;
