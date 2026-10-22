import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
export interface Preferences {
  theme: "auto" | "light" | "dark";
  accent: "blue" | "green" | "purple" | "orange";
  calendarSize: "compact" | "normal" | "large";
  firstDay: number;
  compact: boolean;
  widgetEnabled: boolean;
  widgetOnTop: boolean;
  widgetPosition: "topRight" | "topLeft" | "bottomRight" | "bottomLeft";
  widgetView: "agenda" | "next";
  closeOutside: boolean;
  widgetExpand: boolean;
  widgetUpcomingCount: number;
  notificationsEnabled: boolean;
  catchUpHours: number;
}
export const defaults: Preferences = {
  theme: "auto", accent: "blue", calendarSize: "normal", firstDay: 1, compact: false,
  widgetEnabled: false, widgetOnTop: false, widgetPosition: "topRight", widgetView: "next", closeOutside: true,
  widgetExpand: true, widgetUpcomingCount: 5, notificationsEnabled: true, catchUpHours: 24,
};
export async function loadPreferences(): Promise<{ preferences: Preferences; loadError: string | null }> {
  if (isTauri()) return invoke("get_initial_state");
  return { preferences: { ...defaults }, loadError: null };
}
export async function savePreferences(preferences: Preferences): Promise<void> {
  if (isTauri()) await invoke("save_preferences", { preferences });
}
export async function onPreferencesChange(callback: (preferences: Preferences) => void): Promise<void> {
  if (isTauri()) await listen<Preferences>("preferences-changed", event => callback(event.payload));
}
export async function nativeCommand(command: string, args?: Record<string, unknown>): Promise<void> {
  if (isTauri()) await invoke(command, args);
}

