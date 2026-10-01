// CI-only overlay: no signing secret is ever written into frontend/config files.
import { execFileSync } from "node:child_process";
import { appendFileSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
const pubkey = process.env.TAURI_UPDATER_PUBLIC_KEY?.trim();
const privateKey = process.env.TAURI_SIGNING_PRIVATE_KEY?.trim();
const password = process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD?.trim() ?? "";
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

// Fail in seconds, not after a full build: sign a probe file with the key and
// password, then check the signature's key id matches the public key's.
const keyId = (minisignBase64) => {
  const line = Buffer.from(minisignBase64, "base64").toString("utf8").split("\n")[1];
  return Buffer.from(line, "base64").subarray(2, 10).toString("hex");
};
const probeDir = mkdtempSync(join(tmpdir(), "git-context-sign-probe-"));
try {
  const probe = join(probeDir, "probe.bin");
  writeFileSync(probe, "probe");
  try {
    execFileSync(process.execPath, ["node_modules/@tauri-apps/cli/tauri.js", "signer", "sign", probe], {
      env: { ...process.env, TAURI_SIGNING_PRIVATE_KEY: privateKey, TAURI_SIGNING_PRIVATE_KEY_PASSWORD: password },
      stdio: "pipe",
    });
  } catch (error) {
    const output = `${error.stdout ?? ""}${error.stderr ?? ""}`;
    throw new Error(
      /password/i.test(output)
        ? "TAURI_SIGNING_PRIVATE_KEY_PASSWORD does not unlock TAURI_SIGNING_PRIVATE_KEY. Set the password used with `tauri signer generate`."
        : `TAURI_SIGNING_PRIVATE_KEY could not sign a test file: ${output.trim().split("\n").pop()}`,
    );
  }
  if (keyId(readFileSync(`${probe}.sig`, "utf8")) !== keyId(pubkey)) {
    throw new Error(
      "TAURI_UPDATER_PUBLIC_KEY does not belong to TAURI_SIGNING_PRIVATE_KEY. Use the .pub and .key files from the same `tauri signer generate` run.",
    );
  }
} finally {
  rmSync(probeDir, { recursive: true, force: true });
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
  // Pass the trimmed password on too; a pasted trailing newline breaks it.
  if (/[\r\n]/.test(password)) throw new Error("TAURI_SIGNING_PRIVATE_KEY_PASSWORD must be a single line.");
  appendFileSync(process.env.GITHUB_ENV, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD=${password}\n`);
}
