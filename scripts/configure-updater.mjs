// CI-only overlay: no signing secret is ever written into frontend/config files.
import { appendFileSync, mkdtempSync, writeFileSync } from "node:fs";
import { join } from "node:path";
const pubkey = process.env.TAURI_UPDATER_PUBLIC_KEY?.trim();
const privateKey = process.env.TAURI_SIGNING_PRIVATE_KEY?.trim();
if (!pubkey && !privateKey) {
  console.log("Updater signing not configured; building without signed updates.");
  process.exit(0);
}
if (!pubkey || !privateKey) {
  throw new Error(
    "Set TAURI_UPDATER_PUBLIC_KEY (repository variable) and TAURI_SIGNING_PRIVATE_KEY (repository secret) before releasing signed updates.",
  );
}
// Secrets copied from a text file often have a trailing newline. Trim edges,
// but reject wrapped/corrupt values rather than silently changing key contents.
for (const [name, value] of [["TAURI_UPDATER_PUBLIC_KEY", pubkey], ["TAURI_SIGNING_PRIVATE_KEY", privateKey]]) {
  if (!/^[A-Za-z0-9+/]+={0,2}$/.test(value) || Buffer.from(value, "base64").toString("base64") !== value) {
    throw new Error(`${name} must contain the single-line Base64 contents of its Tauri key file, without quotes or internal whitespace.`);
  }
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

// Pass the normalized key via a private temporary file, never stdout or the
// repository. GitHub-hosted runners discard this file after the job.
if (process.env.GITHUB_ENV && process.env.RUNNER_TEMP) {
  const directory = mkdtempSync(join(process.env.RUNNER_TEMP, "git-context-signing-"));
  const keyPath = join(directory, "updater.key");
  writeFileSync(keyPath, privateKey, { mode: 0o600 });
  appendFileSync(process.env.GITHUB_ENV, `TAURI_SIGNING_PRIVATE_KEY=${keyPath}\n`);
}
