import { invoke } from "@tauri-apps/api/core";
export type Account = { id: string; kind: "offline" | "microsoft"; username: string; uuid: string; active: boolean };
export type Instance = { id: string; name: string; kind: string; mc_version: string; loader: string; loader_version?: string | null; game_dir: string; memory_mb: number; jvm_args: string; java_path: string; width: number; height: number; fullscreen: boolean; server: string; installed: boolean };
export type Settings = { ms_client_id: string; launcher_update_url: string; thm_manifest_url: string; announcements_url: string; java_path: string; default_server: string; slot_urls: Record<string, string>; auto_update: boolean };
export type ModInfo = { file: string; enabled: boolean; size: number; id?: string; name?: string; version?: string; description?: string; depends: string[] };
export type Analysis = { mods: ModInfo[]; duplicates: { id: string; files: string[] }[]; missing: { mod: string; dep: string }[]; incompatible: { mod: string; requires: string; minecraft: string }[] };
export type Slot = { id: string; name: string; source: string; installed: boolean; can_download: boolean; info?: string; link?: string; required?: boolean };
export type Progress = { id: string; label: string; done: number; total: number };

export const THM_SITE = "https://highwaymen.cc";
export const THM_DISCORD = "https://discord.gg/thm";
export const LOGO = "https://highwaymen.cc/images/THM2-transparent.webp";

export const api = {
  accounts: () => invoke<Account[]>("list_accounts"),
  addOffline: (username: string) => invoke<Account>("add_offline", { username }),
  removeAccount: (id: string) => invoke<void>("remove_account", { id }),
  setActive: (id: string) => invoke<void>("set_active_account", { id }),
  msBegin: () => invoke<{ device_code: string; user_code: string; verification_uri: string; interval: number; expires_in: number }>("ms_begin"),
  msPoll: (deviceCode: string) => invoke<Account | null>("ms_poll", { deviceCode }),

  instances: () => invoke<Instance[]>("list_instances"),
  mcVersions: () => invoke<string[]>("list_mc_versions"),
  createInstance: (name: string, mcVersion: string, loader: string) => invoke<Instance>("create_instance", { name, mcVersion, loader }),
  renameInstance: (id: string, name: string) => invoke<void>("rename_instance", { id, name }),
  updateInstance: (instance: Instance) => invoke<void>("update_instance", { instance }),
  deleteInstance: (id: string, deleteFiles: boolean) => invoke<void>("delete_instance", { id, deleteFiles }),
  cloneInstance: (id: string, newName: string) => invoke<Instance>("clone_instance", { id, newName }),
  repairInstance: (id: string) => invoke<void>("repair_instance", { id }),
  exportInstance: (id: string, dest: string) => invoke<void>("export_instance", { id, dest }),
  importInstance: (src: string) => invoke<Instance>("import_instance", { src }),

  install: (id: string) => invoke<void>("install_instance", { id }),
  launch: (id: string) => invoke<void>("launch_instance", { id }),
  kill: (id: string) => invoke<void>("kill_instance", { id }),
  running: () => invoke<string[]>("list_running"),

  analyze: (id: string) => invoke<Analysis>("analyze_mods", { id }),
  addJar: (id: string, src: string) => invoke<string>("add_mod_jar", { id, src }),
  toggleMod: (id: string, file: string, enabled: boolean) => invoke<void>("toggle_mod", { id, file, enabled }),
  deleteMod: (id: string, file: string) => invoke<void>("delete_mod", { id, file }),
  exportMods: (id: string, dest: string) => invoke<void>("export_mods", { id, dest }),
  mrSearch: (id: string, query: string) => invoke<any[]>("modrinth_search", { id, query }),
  mrInstall: (id: string, project: string) => invoke<string[]>("modrinth_install", { id, project }),
  modUpdates: (id: string) => invoke<any[]>("check_mod_updates", { id }),
  applyModUpdate: (id: string, file: string, url: string, sha1: string, filename: string) => invoke<void>("apply_mod_update", { id, file, url, sha1, filename }),

  slots: (id: string) => invoke<Slot[]>("thm_slots", { id }),
  installSlot: (id: string, slot: string) => invoke<string>("thm_install_slot", { id, slot }),
  refreshManifest: () => invoke<any>("refresh_manifest"),

  settings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  activity: () => invoke<{ time: string; text: string }[]>("list_activity"),
  version: () => invoke<string>("launcher_version"),
  status: (address: string) => invoke<any>("server_status", { address }),
  announcements: () => invoke<{ title: string; body: string; date?: string }[]>("announcements"),
  checkUpdates: (id: string) => invoke<any>("check_updates", { id }),
  applyLoaderUpdate: (id: string) => invoke<void>("apply_loader_update", { id }),
};
export const errText = (e: unknown) => (typeof e === "string" ? e : (e as Error)?.message ?? String(e));
