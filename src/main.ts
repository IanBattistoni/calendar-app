import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { agendaRows, escapeHtml, listOccurrences, onEventsChange, getUpcoming, onUpcomingChange, type Upcoming, type Occurrence, type CalendarEvent } from "./events";
import { installWidgetHover, remainingTime, miniEvent } from "./widget";
import { editorIsOpen, installEventEditor } from "./event-editor";
import { dayKey, fromKey, monthDays, shiftDay, monthLabel, fullDateLabel, dayLabel, weekdayLabel } from "./calendar";
import { defaults, loadPreferences, nativeCommand, onPreferencesChange, savePreferences, type Preferences } from "./preferences";

const app = document.querySelector<HTMLElement>("#app")!;
const message = document.querySelector<HTMLElement>("#message")!;
const isWidget = new URLSearchParams(location.search).get("view") === "widget";
const media = matchMedia("(prefers-color-scheme: dark)");
let preferences = { ...defaults };
let selected = dayKey(new Date());
let view = fromKey(selected);
let settings = false;
let section = "appearance";
let saving = false;
let midnightTimer: ReturnType<typeof setTimeout> | undefined;
let lastToday = dayKey(new Date());
let occurrences: Occurrence[] = [];
let loadedRange = "";
let requestId = 0;
let eventsError = "";
let eventsLoading = false;
let upcoming: Upcoming[] = [];
let upcomingError = "";
let upcomingRequest = 0;
let widgetExpanded = false;
let countdownTimer: ReturnType<typeof setTimeout> | undefined;
let notificationError: string | null = null;
const widgetHover = isWidget ? installWidgetHover(app, () => preferences.widgetView === "next" && preferences.widgetExpand, expanded => {
  widgetExpanded = expanded; render();
}, notify) : undefined;

async function refreshUpcoming(): Promise<void> {
  const request = ++upcomingRequest;
  try {
    const result = await getUpcoming();
    if (request !== upcomingRequest) return;
    upcoming = result; upcomingError = "";
  } catch (error) { if (request !== upcomingRequest) return; upcomingError = String(error); }
  if (isWidget) { render(); widgetHover?.refresh(); }
}
function scheduleCountdown(): void {
  clearTimeout(countdownTimer);
  if (!isWidget || document.hidden || !upcoming.length) return;
  countdownTimer = setTimeout(() => {
    app.querySelectorAll<HTMLElement>("[data-countdown]").forEach(node => {node.textContent = remainingTime(Number(node.dataset.countdown));});
    scheduleCountdown();
  }, 60_000 - Date.now() % 60_000 + 20);
}

const eventEditor = installEventEditor({
  selected: () => selected,
  select: date => selectDate(date, false),
  find: id => occurrences.find(item => item.event.id === id)?.event ?? upcoming.find(item => item.event.id === id)?.event,
  changed: () => { loadedRange = ""; render(); void refreshUpcoming(); },
  error: notify,
});

function loadVisibleEvents(): void {
  const grid = monthDays(view.getFullYear(), view.getMonth(), preferences.firstDay);
  const start = isWidget && preferences.widgetView === "next" ? dayKey(new Date()) : dayKey(grid[0]);
  const end = isWidget && preferences.widgetView === "next" ? dayKey(shiftDay(new Date(), 366)) : dayKey(grid[41]);
  const range = `${start}/${end}`;
  if (loadedRange === range) return;
  loadedRange = range;
  eventsLoading = true;
  const currentRequest = ++requestId;
  void listOccurrences(start, end).then(items => {
    if (currentRequest !== requestId) return;
    occurrences = items; eventsError = ""; eventsLoading = false; render();
  }).catch(error => {
    if (currentRequest !== requestId) return;
    eventsError = String(error); occurrences = []; eventsLoading = false; render();
  });
}
function dayAgenda(date: string): string {
  if (eventsLoading) return '<p class="empty-state">Cargando agenda…</p>';
  if (eventsError) return `<div class="empty-state"><p>${escapeHtml(eventsError)}</p><button data-action="retry-events">Reintentar</button></div>`;
  return agendaRows(occurrences.filter(item => item.date === date));
}

const icons: Record<string, string> = {
  previous: '<path d="m8 3-5 5 5 5"/>', next: '<path d="m5 3 5 5-5 5"/>',
  settings: '<path d="M8 1v2m0 10v2M1 8h2m10 0h2M3 3l1.5 1.5m7 7L13 13M3 13l1.5-1.5m7-7L13 3"/><circle cx="8" cy="8" r="4"/><circle cx="8" cy="8" r="1"/>',
  close: '<path d="m4 4 8 8m0-8-8 8"/>',
  calendar: '<rect x="2" y="3" width="12" height="11" rx="2"/><path d="M2 7h12M5 1v4m6-4v4"/>',
};
function icon(name: string): string {
  return `<svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${icons[name] ?? ""}</svg>`;
}
function button(action: string, label: string, glyph: string): string {
  return `<button class="icon-button" data-action="${action}" aria-label="${label}" title="${label}">${icon(glyph)}</button>`;
}
function notify(error: unknown): void {
  message.textContent = error instanceof Error ? error.message : String(error);
  message.hidden = false;
}
function applyTheme(): void {
  document.documentElement.dataset.theme = preferences.theme === "auto" ? (media.matches ? "dark" : "light") : preferences.theme;
  document.documentElement.dataset.accent = preferences.accent;
  document.documentElement.dataset.size = preferences.calendarSize;
  document.documentElement.dataset.density = preferences.compact ? "compact" : "comfortable";
}
function renderCalendar(): void {
  const today = dayKey(new Date());
  const days = monthDays(view.getFullYear(), view.getMonth(), preferences.firstDay);
  const weekdayNames = Array.from({ length: 7 }, (_, i) => weekdayLabel.format(new Date(2026, 0, 4 + ((i + preferences.firstDay) % 7), 12)).replace(".", ""));
  app.innerHTML = `
    <article class="calendar-panel">
      <header class="panel-header">
        <button class="today-label" data-action="today" title="Ir a hoy">${dayLabel.format(new Date())}</button>
        ${button("hide", "Ocultar calendario", "close")}
      </header>
      <section class="month-section" aria-label="Calendario mensual">
        <div class="month-heading">
          <h1 aria-live="polite">${monthLabel.format(view)}</h1>
          <nav aria-label="Cambiar mes">${button("previous", "Mes anterior", "previous")}${button("next", "Mes siguiente", "next")}</nav>
        </div>
        <div class="weekdays" aria-hidden="true">${weekdayNames.map(name => `<span>${name}</span>`).join("")}</div>
        <div class="month-grid" role="group" aria-label="${monthLabel.format(view)}">
          ${days.map(date => {
            const key = dayKey(date);
            return `<button class="day ${date.getMonth() !== view.getMonth() ? "outside" : ""} ${key === today ? "today" : ""} ${key === selected ? "selected" : ""} ${occurrences.some(item => item.date === key) ? "has-events" : ""}" data-date="${key}" aria-label="${fullDateLabel.format(date)}${occurrences.some(item => item.date === key) ? ", con eventos" : ""}" aria-pressed="${key === selected}" ${key === today ? 'aria-current="date"' : ""} tabindex="${key === selected ? "0" : "-1"}"><span>${date.getDate()}</span></button>`;
          }).join("")}
        </div>
      </section>
      <section class="agenda" aria-labelledby="agenda-title">
        <div class="agenda-heading"><h2 id="agenda-title">${selected === today ? "Hoy" : dayLabel.format(fromKey(selected))}</h2><button class="icon-button" data-add-event aria-label="Agregar evento" title="Agregar evento">＋</button></div>
        ${dayAgenda(selected)}
      </section>
      <footer class="panel-footer">
        <button data-action="toggle-widget">${preferences.widgetEnabled ? "Ocultar widget" : "Mostrar widget"}</button>
        <button data-action="settings">${icon("settings")} Configuración</button>
      </footer>
    </article>`;
}
function options(values: [string, string][], current: string | number): string {
  return values.map(([value, title]) => `<option value="${value}" ${String(current) === value ? "selected" : ""}>${title}</option>`).join("");
}
function selectRow(key: keyof Preferences, title: string, description: string, values: [string, string][]): string {
  return `<label class="setting-row"><span><span class="setting-title">${title}</span><small>${description}</small></span><select data-pref="${key}" ${saving ? "disabled" : ""}>${options(values, String(preferences[key]))}</select></label>`;
}
function toggleRow(key: keyof Preferences, title: string, description: string): string {
  return `<label class="setting-row"><span><span class="setting-title">${title}</span><small>${description}</small></span><input class="switch" type="checkbox" role="switch" data-pref="${key}" ${preferences[key] ? "checked" : ""} ${saving ? "disabled" : ""}/></label>`;
}
function renderSettings(): void {
  const pages = [["appearance", "Apariencia"], ["widget", "Widget"], ["notifications", "Avisos"], ["behavior", "Comportamiento"]];
  const title = pages.find(([key]) => key === section)![1];
  let rows = "";
  if (section === "appearance") {
    rows = `<h2>Personalización</h2>` +
      selectRow("theme", "Tema", "Automático sigue el modo de Windows.", [["auto", "Automático"], ["light", "Claro"], ["dark", "Oscuro"]]) +
      selectRow("accent", "Color de énfasis", "Para la fecha actual y los controles.", [["blue", "Azul"], ["green", "Verde"], ["purple", "Morado"], ["orange", "Naranja"]]) +
      `<h2>Tamaño y distribución</h2>` +
      selectRow("calendarSize", "Calendario emergente", "Tamaño de la ventana y de las fechas.", [["compact", "Compacto"], ["normal", "Normal"], ["large", "Grande"]]) +
      selectRow("firstDay", "Primer día de la semana", "Orden de los días del calendario.", [["1", "Lunes"], ["0", "Domingo"]]) +
      toggleRow("compact", "Agenda compacta", "Reduce el espacio de las filas de la agenda.");
  } else if (section === "widget") {
    rows = toggleRow("widgetEnabled", "Mostrar widget", "Opcional. Ocultarlo mantiene disponible el icono junto al reloj.") +
      selectRow("widgetView", "Contenido del widget", "Agenda fija o tarjeta compacta desplegable.", [["next", "Próximo evento"], ["agenda", "Agenda semanal"]]) +
      (preferences.widgetView === "next" ? toggleRow("widgetExpand", "Expandir al pasar el mouse", "Despliega los eventos que vienen después del primero.") +
      selectRow("widgetUpcomingCount", "Elementos próximos", "Máximo de elementos después del próximo evento.", [["3", "3 elementos"], ["5", "5 elementos"], ["8", "8 elementos"]]) : "") +
      selectRow("widgetPosition", "Posición", "Puedes moverlo arrastrando su encabezado.", [["topRight", "Arriba a la derecha"], ["topLeft", "Arriba a la izquierda"], ["bottomRight", "Abajo a la derecha"], ["bottomLeft", "Abajo a la izquierda"]]) +
      toggleRow("widgetOnTop", "Siempre visible", "Mantenerlo encima de otras ventanas.") +
      '<p class="note">Sin «Siempre visible», el widget se comporta como una ventana normal y otras aplicaciones pueden cubrirlo.</p>';
  } else if (section === "notifications") {
    rows = toggleRow("notificationsEnabled", "Avisos de Windows", "Se ejecutan en segundo plano mientras Calendario está en la bandeja.") +
      selectRow("catchUpHours", "Recuperar avisos pendientes", "Al iniciar o reanudar el PC, recupera el aviso más reciente de cada evento.", [["0", "Solo puntuales"], ["1", "Última hora"], ["24", "Últimas 24 horas"]]) +
      '<div class="setting-row"><span><span class="setting-title">Probar notificación</span><small>Envía una prueba para comprobar los avisos en este equipo.</small></span><button class="test-notification" data-action="test-notification">Enviar prueba</button></div>' +
      `<p class="note">La anticipación y la hora de los días completos se eligen en cada evento. Los eventos existentes conservan sus datos y no enviarán avisos hasta que los configures.</p><p class="note">Windows puede silenciar los avisos con «No molestar». La versión instalada identifica los avisos como Calendario; en desarrollo pueden aparecer con otra identidad.</p>${notificationError ? `<p class="form-error" role="alert">${escapeHtml(notificationError)}</p>` : ""}`;
  } else {
    rows = '<div class="setting-row"><span><span class="setting-title">Icono junto al reloj</span><small>Permanece disponible mientras Calendario está en ejecución.</small></span><span class="badge">Siempre activo</span></div>' +
      toggleRow("closeOutside", "Ocultar al hacer clic fuera", "Oculta el calendario o la configuración al hacer clic en otra ventana.") +
      '<div class="setting-row"><span><span class="setting-title">Cerrar la ventana</span><small>Oculta el calendario. Para terminar el proceso, usa «Salir de Calendario» en el icono de la bandeja.</small></span></div>';
  }
  app.innerHTML = `<article class="settings-panel">
    <header class="panel-header"><span class="app-name">${icon("calendar")} Calendario · Configuración</span>${button("back", "Volver al calendario", "close")}</header>
    <div class="settings-layout"><nav class="settings-nav" aria-label="Configuración">${pages.map(([key, label]) => `<button data-section="${key}" aria-current="${section === key ? "page" : "false"}">${label}</button>`).join("")}<small>Local. Simple. Tuyo.</small></nav>
    <section class="settings-content"><h1>${title}</h1>${rows}<p class="save-note" role="status">${saving ? "Guardando…" : isTauri() ? "Los cambios se guardan automáticamente en este equipo." : "Vista previa en navegador. Los cambios no se guardan."}</p></section></div>
  </article>`;
}
function renderWidget(): void {
  const nextView = preferences.widgetView === "next";
  const date = fromKey(selected);
  const next = upcoming[0];
  if (nextView) {
    app.innerHTML = `<article class="widget-panel next-widget ${widgetExpanded ? "expanded" : ""}">
      <div class="mini-summary"><div class="mini-label" data-action="drag" tabindex="0" data-mini-toggle title="Arrastrar para mover">Próximo evento</div>
      ${upcomingError ? `<p class="mini-error">${escapeHtml(upcomingError)}</p>` : next ? miniEvent(next, true) : '<div class="mini-empty"><strong>Sin próximos eventos</strong><small>Tu agenda está al día.</small></div>'}</div>
      <section class="mini-rest" ${widgetExpanded ? "" : "hidden"} aria-label="Después"><h2>Después</h2><div class="upcoming-scroll">${upcoming.length > 1 ? upcoming.slice(1, preferences.widgetUpcomingCount + 1).map(item => miniEvent(item)).join("") : '<p class="mini-empty">No hay más eventos próximos.</p>'}</div></section>
    </article>`;
    return;
  }
  const start = shiftDay(date, -((date.getDay() - preferences.firstDay + 7) % 7));
  app.innerHTML = `<article class="widget-panel agenda-widget">
    <header class="panel-header"><button class="drag-handle" data-action="drag" title="Arrastrar widget">${monthLabel.format(date)}</button>${button("open-calendar", "Abrir calendario", "calendar")}</header>
    <div class="week-navigation">${button("week-previous", "Semana anterior", "previous")}<div class="week-strip">${Array.from({ length: 7 }, (_, i) => shiftDay(start, i)).map(d => `<button data-date="${dayKey(d)}" class="${dayKey(d) === selected ? "active" : ""}" aria-label="${fullDateLabel.format(d)}"><small>${weekdayLabel.format(d).replace(".", "")}</small><span>${d.getDate()}</span></button>`).join("")}</div>${button("week-next", "Semana siguiente", "next")}</div>
    <div class="agenda-heading"><h2>${dayLabel.format(date)}</h2><button class="icon-button" data-add-event aria-label="Agregar evento">＋</button></div><section class="widget-agenda">${dayAgenda(selected)}</section>
  </article>`;
}
function render(): void {
  loadVisibleEvents();
  applyTheme();
  document.body.classList.toggle("widget", isWidget);
  document.body.classList.toggle("preview", !isTauri());
  if (isWidget) renderWidget(); else if (settings) renderSettings(); else renderCalendar();
  scheduleCountdown();
}
async function updatePreferences(next: Preferences): Promise<void> {
  if (saving) return;
  saving = true;
  if (settings) render();
  try {
    await savePreferences(next);
    preferences = next;
    widgetHover?.reset();
    message.hidden = true;
  } catch (error) { notify("No se pudo guardar la configuración: " + String(error)); }
  finally { saving = false; render(); }
}
async function setSettings(open: boolean): Promise<void> {
  await nativeCommand("set_panel_mode", { settings: open });
  settings = open;
  message.hidden = true;
  render();
  app.querySelector<HTMLElement>(open ? ".settings-nav button" : ".day.selected")?.focus();
}
function selectDate(key: string, focus: boolean): void {
  selected = key;
  view = fromKey(key);
  render();
  if (focus) app.querySelector<HTMLElement>(`[data-date="${key}"]`)?.focus();
}
app.addEventListener("click", event => {
  const target = (event.target as Element).closest<HTMLElement>("button");
  if (!target) return;
  const date = target.dataset.date;
  if (date) { selectDate(date, true); return; }
  if (target.dataset.section) { section = target.dataset.section; render(); app.querySelector<HTMLElement>(`[data-section="${section}"]`)?.focus(); return; }
  void (async () => {
    switch (target.dataset.action) {
      case "week-previous": case "week-next": selectDate(dayKey(shiftDay(fromKey(selected), target.dataset.action === "week-next" ? 7 : -7)), false); break;
      case "today": selectDate(dayKey(new Date()), true); break;
      case "previous": case "next":
        view = new Date(view.getFullYear(), view.getMonth() + (target.dataset.action === "next" ? 1 : -1), 1, 12);
        selected = dayKey(view); render(); app.querySelector<HTMLElement>(`[data-action="${target.dataset.action}"]`)?.focus(); break;
      case "settings": await setSettings(true); break;
      case "back": await setSettings(false); break;
      case "hide": await nativeCommand("hide_panel"); break;
      case "toggle-widget": await updatePreferences({ ...preferences, widgetEnabled: !preferences.widgetEnabled }); break;
      case "retry-events": loadedRange = ""; render(); break;
      case "test-notification":
        await nativeCommand("test_notification");
        notify(isTauri() ? "Prueba enviada. Revisa el aviso o el centro de notificaciones de Windows." : "Abre la aplicación de escritorio para probar los avisos.");
        break;
      case "open-calendar": await nativeCommand("open_calendar"); break;
      case "open-event": await nativeCommand("open_event_in_calendar", { id: target.dataset.openEventId }); break;
    }
  })().catch(notify);
});
app.addEventListener("pointerdown", event => {
  if (event.button === 0 && (event.target as Element).closest('[data-action="drag"]')) {
    event.preventDefault();
    widgetHover?.drag(true);
    void nativeCommand("start_widget_drag").catch(error => { widgetHover?.drag(false); notify(error); });
  }
});
app.addEventListener("change", event => {
  const input = event.target as HTMLInputElement | HTMLSelectElement;
  const key = input.dataset.pref as keyof Preferences | undefined;
  if (!key) return;
  const value = input instanceof HTMLInputElement ? input.checked : ["firstDay", "widgetUpcomingCount", "catchUpHours"].includes(key) ? Number(input.value) : input.value;
  void updatePreferences({ ...preferences, [key]: value });
});
document.addEventListener("keydown", event => {
  if (editorIsOpen()) return;
  if (event.key === "Escape") {
    if (isWidget) return;
    void (settings ? setSettings(false) : nativeCommand("hide_panel")).catch(notify);
    return;
  }
  const target = (event.target as Element).closest<HTMLElement>("[data-date]");
  if (!target || isWidget) return;
  const offset: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -7, ArrowDown: 7 };
  if (event.key in offset) {
    event.preventDefault(); selectDate(dayKey(shiftDay(fromKey(selected), offset[event.key])), true);
  } else if (event.key === "Home") {
    event.preventDefault(); selectDate(dayKey(new Date()), true);
  }
});
function scheduleMidnight(): void {
  clearTimeout(midnightTimer);
  if (document.hidden) return;
  const now = new Date();
  const next = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
  midnightTimer = setTimeout(() => { refreshToday(); scheduleMidnight(); }, next.getTime() - now.getTime() + 100);
}
function refreshToday(): void {
  const today = dayKey(new Date());
  if (today !== lastToday) {
    if (selected === lastToday) { selected = today; view = fromKey(today); }
    lastToday = today; render();
  }
}
document.addEventListener("visibilitychange", () => {
  if (!document.hidden) {refreshToday(); if (isWidget) void refreshUpcoming();}
  scheduleCountdown();
  scheduleMidnight();
});
window.addEventListener("focus", () => { refreshToday(); scheduleMidnight(); });
media.addEventListener("change", applyTheme);
async function start(): Promise<void> {
  await onEventsChange(() => { loadedRange = ""; render(); void refreshUpcoming(); });
  await onUpcomingChange(() => {if (isWidget) void refreshUpcoming(); refreshToday();});
  await onPreferencesChange(next => { preferences = next; render(); widgetHover?.reset(); });
  if (isTauri()) {
    if (!isWidget) await listen<CalendarEvent>("edit-event", event => {
      void (async () => { await setSettings(false); selectDate(event.payload.date, false); await eventEditor.open(event.payload); })().catch(notify);
    });
    if (isWidget) await listen<boolean>("widget-dragging", event => widgetHover?.drag(event.payload));
    await listen<string | null>("notification-status", event => {notificationError = event.payload; if (settings && section === "notifications") render();});
    notificationError = await invoke<string | null>("notification_status");
  }
  const initial = await loadPreferences();
  preferences = initial.preferences;
  if (!isWidget) {
    await nativeCommand("set_panel_mode", { settings: false });
    await nativeCommand("open_calendar");
  }
  render();
  if (isWidget) { widgetHover?.refresh(); void refreshUpcoming(); }
  if (initial.loadError) notify(initial.loadError + ". Se muestran los valores iniciales; revisa el archivo antes de guardar cambios.");
  scheduleMidnight();
}
void start().catch(error => { render(); notify(error); });




