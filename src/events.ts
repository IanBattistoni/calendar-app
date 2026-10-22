import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
export type EventKind = "event" | "birthday" | "reminder";
export interface CalendarEvent {
  id: string;
  title: string;
  date: string;
  time: string | null;
  description: string;
  kind: EventKind;
  annual: boolean;
  weekly?: { days: number[]; until: string | null } | null;
  reminders: number[];
  notifyAt: string;
}
export interface Upcoming {
  event: CalendarEvent;
  date: string;
  eventAt: number;
  remindAt: number | null;
  expiresAt: number;
}
export async function getUpcoming(): Promise<Upcoming[]> {
  return isTauri() ? invoke("get_upcoming") : [];
}
export async function onUpcomingChange(callback: () => void): Promise<void> {
  if (isTauri()) {
    await listen("upcoming-changed", callback);
    await listen("clock-changed", callback);
  }
}
export interface Occurrence { event: CalendarEvent; date: string }
export const kindLabels: Record<EventKind, string> = { event: "Evento", birthday: "Cumpleaños", reminder: "Recordatorio" };
export function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]!);
}
export async function listOccurrences(start: string, end: string): Promise<Occurrence[]> {
  return isTauri() ? invoke("list_occurrences", { start, end }) : [];
}
export async function saveEvent(event: CalendarEvent): Promise<CalendarEvent> {
  if (!isTauri()) throw Error("Abre la aplicación de escritorio para guardar tus eventos.");
  return invoke("save_event", { event });
}
export async function deleteEvent(id: string): Promise<void> {
  if (!isTauri()) throw Error("Abre la aplicación de escritorio para modificar tus eventos.");
  await invoke("delete_event", { id });
}
export async function onEventsChange(callback: () => void): Promise<void> {
  if (isTauri()) await listen("events-changed", callback);
}
export function agendaRows(items: Occurrence[]): string {
  if (!items.length) return '<div class="empty-state"><p>Sin eventos para este día</p></div>';
  return items.map(({ event }) => `<button class="event-row" data-event-id="${escapeHtml(event.id)}" aria-label="Editar ${escapeHtml(event.title)}">
    <span class="event-time">${event.time ?? "Todo el día"}</span>
    <span class="event-info"><strong>${escapeHtml(event.title)}</strong><small>${kindLabels[event.kind]}${event.weekly ? " · Semanal" : event.annual ? " · Anual" : ""}</small></span>
    <span class="event-marker ${event.kind}" aria-hidden="true"></span></button>`).join("");
}

