import { useEffect, useState } from "react";
import { api } from "../lib/api";
import { message } from "../lib/utils";
import type { AccountCheck, GitProvider } from "../lib/types";

export function useAccountCheck(
  provider: GitProvider,
  host: string,
  username: string,
  delay = 400,
) {
  const [check, setCheck] = useState<AccountCheck | null>(null);
  const [checking, setChecking] = useState(false);
  const name = username.trim().replace(/^@/, "");

  useEffect(() => {
    setCheck(null);
    if (!name || !host) {
      setChecking(false);
      return;
    }
    let cancelled = false;
    setChecking(true);
    const timer = setTimeout(() => {
      api
        .checkAccount(provider, host, name)
        .catch((e): AccountCheck => ({
          status: "unknown",
          username: name,
          displayName: null,
          profileUrl: null,
          message: message(e),
        }))
        .then((result) => {
          if (cancelled) return;
          setCheck(result);
          setChecking(false);
        });
    }, delay);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [provider, host, name, delay]);

  return { check, checking };
}
