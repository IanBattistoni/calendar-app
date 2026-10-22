import { escapeHtml, type Upcoming } from "./events.ts";
import { dayLabel, fromKey } from "./calendar.ts";

export function remainingTime(target: number, now = Date.now()): string {
  const minutes = Math.ceil((target - now) / 60_000);
  if (minutes <= 0) return "Ahora";
  if (minutes < 60) return `En ${minutes} min`;
  if (minutes < 1440) return `En ${Math.floor(minutes / 60)} h${minutes % 60 ? " " + minutes % 60 + " min" : ""}`;
  const days = Math.ceil(minutes / 1440);
  return `En ${days} ${days === 1 ? "día" : "días"}`;
}
export function upcomingRows(items: Upcoming[], count: number): string {
  if (!items.length) return '<p class="widget-empty">No tienes próximos eventos.</p>';
  return items.slice(0, count).map(item => {
    const target = item.remindAt ?? item.eventAt;
    const time = new Intl.DateTimeFormat("es-CL", { hour: "2-digit", minute: "2-digit", hour12: false }).format(target);
    return `<button class="upcoming-row" data-event-id="${escapeHtml(item.event.id)}">
      <span class="status-dot"></span><span class="event-info"><strong>${escapeHtml(item.event.title)}</strong>
      <small>${item.remindAt ? "Aviso" : "Evento"} · ${dayLabel.format(fromKey(dayKeyLocal(target)))} · ${item.event.time || item.remindAt ? time : "Todo el día"}</small>
      <span class="countdown" data-countdown="${target}">${remainingTime(target)}</span></span></button>`;
  }).join("");
}
export function miniEvent(item: Upcoming, primary = false): string {
  return `<button class="mini-event ${primary ? "primary-event" : ""}" data-action="open-event" data-open-event-id="${escapeHtml(item.event.id)}"><strong>${escapeHtml(item.event.title)}</strong><small><span data-countdown="${item.eventAt}">${remainingTime(item.eventAt)}</span> · ${dayLabel.format(fromKey(item.date))}, ${item.event.time ?? "Todo el día"}</small></button>`;
}
function dayKeyLocal(timestamp: number): string {
  const date=new Date(timestamp);
  return [date.getFullYear(),String(date.getMonth()+1).padStart(2,"0"),String(date.getDate()).padStart(2,"0")].join("-");
}
