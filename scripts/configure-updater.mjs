// CI-only overlay: no signing secret is ever written into frontend/config files.
import { writeFileSync } from "node:fs";
const pubkey = process.env.TAURI_UPDATER_PUBLIC_KEY?.trim();
if (!pubkey || !process.env.TAURI_SIGNING_PRIVATE_KEY) {
  throw new Error(
    "Set TAURI_UPDATER_PUBLIC_KEY (repository variable) and TAURI_SIGNING_PRIVATE_KEY (repository secret) before releasing signed updates.",
  );
}
const decoded = Buffer.from(pubkey, "base64").toString("utf8");
if (!decoded.startsWith("untrusted comment:") || !decoded.includes("\nRW")) {
  throw new Error(
    "TAURI_UPDATER_PUBLIC_KEY must be the contents of the Tauri signer .pub file.",
  );
}
writeFileSync(
  "src-tauri/tauri.updater.conf.json",
  JSON.stringify(
    {
      bundle: { createUpdaterArtifacts: true },
      plugins: { updater: { pubkey } },
    },
    null,
    2,
  ),
);
