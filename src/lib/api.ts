import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  AppData,
  Detection,
  Plan,
  Profile,
  RepoStatus,
  Settings,
  SshKey,
  Verification,
} from "./types";
export const native = isTauri();
function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!native)
    return Promise.reject(
      new Error(
        "Run the desktop application to access Git and SSH. Browser preview has no system access.",
      ),
    );
  return invoke<T>(command, args);
}
export const api = {
  renameProfile: (profileId: string, name: string) =>
    call<AppData>("rename_profile", { profileId, name }),
  removeProfile: (profileId: string) =>
    call<AppData>("remove_profile", { profileId }),
  updateRepository: (repositoryId: string, name: string, path: string) =>
    call<AppData>("update_repository", { repositoryId, name, path }),
  removeRepository: (repositoryId: string) =>
    call<AppData>("remove_repository", { repositoryId }),
  registerKey: (publicPath: string) =>
    call<AppData>("register_key", { publicPath }),
  removeKeyReference: (publicPath: string) =>
    call<AppData>("remove_key_reference", { publicPath }),
  snapshot: () => call<AppData>("snapshot"),
  detect: () => call<Detection>("detect_environment"),
  importKey: (publicPath: string) => call<SshKey>("import_key", { publicPath }),
  createProfile: (profile: Profile) =>
    call<AppData>("create_profile", { profile }),
  updateProfile: (profile: Profile) =>
    call<AppData>("update_profile", { profile }),
  choosePublicKey: () => call<string | null>("choose_public_key"),
  openFolder: (repositoryId: string) =>
    call<void>("open_repository_folder", { repositoryId }),
  selectProfile: (profileId: string) =>
    call<AppData>("select_profile", { profileId }),
  register: (path: string) => call<AppData>("register_repository", { path }),
  chooseRepositoryFolder: () => call<string | null>("choose_repository_folder"),
  status: (repositoryId: string) =>
    call<RepoStatus>("repository_status", { repositoryId }),
  plan: (repositoryId: string, profileId: string, rewriteRemote: boolean) =>
    call<Plan>("plan_assignment", { repositoryId, profileId, rewriteRemote }),
  activateProfile: (profileId: string) =>
    call<AppData>("activate_profile", { profileId }),
  planActivation: (profileId: string) =>
    call<Plan>("plan_activation", { profileId }),
  planGlobal: (profileId: string) =>
    call<Plan>("plan_global_profile", { profileId }),
  apply: (planId: string) => call<string>("apply_assignment", { planId }),
  transactions: () => call<[string, string][]>("list_transactions"),
  undo: (transactionId: string) =>
    call<AppData>("undo_assignment", { transactionId }),
  verify: (profileId: string) =>
    call<Verification>("verify_profile", { profileId }),
  launch: (repositoryId: string, profileId: string, kind: string) =>
    call<void>("launch_terminal", { repositoryId, profileId, kind }),
  settings: (settings: Settings) =>
    call<AppData>("save_settings", { settings }),
};
export const emptyData: AppData = {
  schemaVersion: 2,
  profiles: [],
  repositories: [],
  activeProfileId: null,
  settings: { theme: "dark", startupView: "dashboard" },
};
