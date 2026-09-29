import { useState } from "react";
import type { Workspace } from "../hooks/use-workspace";
import type { Profile, Repository, SshKey } from "../lib/types";
import { api } from "../lib/api";
import { Modal } from "./ui/dialog";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Field } from "./shared";
export type ManagementAction =
  | { kind: "rename-profile" | "remove-profile"; profile: Profile }
  | { kind: "edit-repository" | "remove-repository"; repository: Repository }
  | { kind: "remove-key"; key: SshKey };
export function ManageDialog({
  action,
  workspace: w,
  close,
}: {
  action: ManagementAction;
  workspace: Workspace;
  close: () => void;
}) {
  const [name, setName] = useState(
    "profile" in action
      ? action.profile.name
      : "repository" in action
        ? action.repository.name
        : "",
  );
  const [path, setPath] = useState(
    "repository" in action ? action.repository.path : "",
  );
  const editing =
    action.kind === "rename-profile" || action.kind === "edit-repository";
  const label =
    "profile" in action
      ? action.profile.name
      : "repository" in action
        ? action.repository.name
        : action.key.publicPath;
  const dependencies =
    action.kind === "remove-profile"
      ? w.data.repositories
          .filter((r) => r.profileId === action.profile.id)
          .map((r) => r.name)
      : action.kind === "remove-key"
        ? w.data.profiles
            .filter((p) => p.publicKeyPath === action.key.publicPath)
            .map((p) => p.name)
        : [];
  const isGlobal =
    action.kind === "remove-profile" &&
    w.data.globalProfileId === action.profile.id;
  const blocked = isGlobal || dependencies.length > 0;
  const description =
    action.kind === "remove-repository"
      ? "Remove this registration from Git Context. The folder, Git configuration, remotes and SSH configuration stay on disk. You can add the folder again."
      : action.kind === "remove-profile"
        ? "Remove this saved profile. Existing SSH configuration and key files stay on disk; this does not revoke account access or undo previous configuration changes."
        : action.kind === "remove-key"
          ? "Remove this key reference from the app and hide it from automatic discovery. No key files are deleted and no SSH-agent or provider access is revoked. Import the .pub file to show it again."
          : action.kind === "edit-repository"
            ? "Update the display name or choose a new repository location. A different repository clears this registration’s profile assignment; existing files and configuration stay unchanged."
            : "Change the display name. Git author, SSH identity and repository assignments stay unchanged.";
  return (
    <Modal
      open
      onOpenChange={(open) => {
        if (!open && !w.busy) close();
      }}
      title={
        editing
          ? action.kind === "rename-profile"
            ? "Rename profile"
            : "Edit repository"
          : "Remove from Git Context?"
      }
      description={description}
    >
      <form
        className="form-stack"
        onSubmit={(e) => {
          e.preventDefault();
          if (blocked) return;
          void w.run(editing ? "Saving changes" : "Removing item", async () => {
            let data;
            switch (action.kind) {
              case "rename-profile":
                data = await api.renameProfile(action.profile.id, name);
                break;
              case "remove-profile":
                data = await api.removeProfile(action.profile.id);
                break;
              case "edit-repository":
                data = await api.updateRepository(
                  action.repository.id,
                  name,
                  path,
                );
                break;
              case "remove-repository":
                data = await api.removeRepository(action.repository.id);
                break;
              case "remove-key":
                data = await api.removeKeyReference(action.key.publicPath);
                break;
            }
            w.setData(data);
            await w.refreshStatuses(data);
            close();
            w.setNotice({
              error: false,
              text: editing
                ? "Changes saved."
                : "Removed from Git Context. Files and Git/SSH configuration are unchanged.",
            });
          });
        }}
      >
        {editing ? (
          <Field label="Display name">
            <Input
              required
              maxLength={200}
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </Field>
        ) : (
          <p className="mono" style={{ overflowWrap: "anywhere" }}>
            {label}
          </p>
        )}
        {action.kind === "edit-repository" && (
          <Field label="Repository folder">
            <div className="row">
              <Input
                required
                value={path}
                onChange={(e) => setPath(e.target.value)}
              />
              <Button
                variant="secondary"
                disabled={!!w.busy}
                onClick={() =>
                  void w.run("Choosing folder", async () => {
                    const folder = await api.chooseRepositoryFolder();
                    if (folder) setPath(folder);
                  })
                }
              >
                Choose folder
              </Button>
            </div>
          </Field>
        )}
        {isGlobal && (
          <p className="inline-error">
            This is the global default. Set another global default, or restore
            its transaction in Settings, before removing it.
          </p>
        )}
        {!!dependencies.length && (
          <p className="inline-error">
            Used by: {dependencies.join(", ")}.{" "}
            {action.kind === "remove-key"
              ? "Change or remove these profiles first."
              : "Activate a profile for this provider to retire its old repository assignments first."}
          </p>
        )}
        <div className="form-actions">
          <Button variant="secondary" disabled={!!w.busy} onClick={close}>
            Cancel
          </Button>
          <Button
            type="submit"
            variant={editing ? "default" : "destructive"}
            disabled={!!w.busy || blocked}
          >
            {editing ? "Save changes" : "Remove from app"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}
