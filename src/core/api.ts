import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Alert,
  CoreEvent,
  DisplayConfig,
  Entry,
  EntryView,
  Group,
  PushInfo,
  Recap,
  SourceCfg,
  TestResult,
} from "./types";

export type Secrets = [string, string][];

export const api = {
  listEntries: () => invoke<EntryView[]>("list_entries"),
  saveEntry: (entry: Entry, secrets: Secrets) => invoke<Entry>("save_entry", { entry, secrets }),
  deleteEntry: (id: number) => invoke<void>("delete_entry", { id }),
  reorderEntries: (ids: number[]) => invoke<void>("reorder_entries", { ids }),
  listGroups: () => invoke<Group[]>("list_groups"),
  saveGroup: (group: Group) => invoke<Group>("save_group", { group }),
  deleteGroup: (id: number) => invoke<void>("delete_group", { id }),
  listAlerts: (entryId: number) => invoke<Alert[]>("list_alerts", { entryId }),
  saveAlert: (alert: Alert) => invoke<Alert>("save_alert", { alert }),
  deleteAlert: (id: number) => invoke<void>("delete_alert", { id }),
  testSource: (source: SourceCfg, secrets: Secrets) => invoke<TestResult>("test_source", { source, secrets }),
  fetchFavicon: (url: string) => invoke<string>("fetch_favicon", { url }),
  entryHistory: (entryId: number, days: number) => invoke<[number, number][]>("entry_history", { entryId, days }),
  simulateChange: (entryId: number, value: number) => invoke<void>("simulate_change", { entryId, value }),
  weeklyRecap: () => invoke<Recap>("weekly_recap"),
  showRecap: () => invoke<void>("show_recap"),
  saveSnapshot: (name: string, png: string) => invoke<string>("save_snapshot", { name, png }),
  exportCsv: (entryId: number | null) => invoke<string>("export_csv", { entryId }),
  keyringAvailable: () => invoke<boolean>("keyring_available"),
  hasSecrets: (names: string[]) => invoke<boolean[]>("has_secrets", { names }),
  pushInfo: () => invoke<PushInfo>("push_info"),
  regeneratePushToken: () => invoke<PushInfo>("regenerate_push_token"),
  restartPush: () => invoke<PushInfo>("restart_push"),
  getDisplay: () => invoke<DisplayConfig>("get_display"),
  saveDisplay: (cfg: DisplayConfig) => invoke<DisplayConfig>("save_display", { cfg }),
  widgetResize: (width: number, height: number, show: boolean) =>
    invoke<void>("widget_resize", { width, height, show }),
  widgetHit: (rect: { x: number; y: number; w: number; h: number }) => invoke<void>("widget_hit", { rect }),
  widgetDone: () => invoke<void>("widget_done"),
  openSettings: () => invoke<void>("open_settings"),
  refreshAll: () => invoke<void>("refresh_all"),
  platformInfo: () => invoke<{ os: string; version: string }>("platform_info"),
  quit: () => invoke<void>("quit_app"),
  listMonitors: () => invoke<string[]>("list_monitors"),
};

export function onCoreEvent(cb: (e: CoreEvent) => void): Promise<UnlistenFn> {
  return listen<CoreEvent>("core-event", (ev) => cb(ev.payload));
}

export function onDisplayChanged(cb: (c: DisplayConfig) => void): Promise<UnlistenFn> {
  return listen<DisplayConfig>("display-changed", (ev) => cb(ev.payload));
}

export function onPushChanged(cb: (p: PushInfo) => void): Promise<UnlistenFn> {
  return listen<PushInfo>("push-changed", (ev) => cb(ev.payload));
}

export function onReordered(cb: (ids: number[]) => void): Promise<UnlistenFn> {
  return listen<number[]>("entries-reordered", (ev) => cb(ev.payload));
}

export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}
