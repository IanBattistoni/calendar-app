import { type CalendarEvent, type EventKind, deleteEvent, escapeHtml, kindLabels, saveEvent } from "./events";
import { nativeCommand } from "./preferences";
import { fullDateLabel, fromKey } from "./calendar";

export interface EditorContext {
  selected: () => string;
  select: (date: string) => void;
  find: (id: string) => CalendarEvent | undefined;
  changed: () => void;
  error: (error: unknown) => void;
}
let dialog: HTMLDialogElement | undefined;
let menu: HTMLElement | undefined;
let busy = false;
export function editorIsOpen(): boolean { return !!dialog?.open; }
function closeMenu(): void { menu?.remove(); menu = undefined; window.dispatchEvent(new Event("calendar-overlay-closed")); }
export function installEventEditor(context: EditorContext) {
  async function openEditor(kind: EventKind, existing?: CalendarEvent): Promise<void> {
    closeMenu();
    if (dialog?.open) return;
    await nativeCommand("set_editor_open", { open: true });
    const event: CalendarEvent = existing ?? { id: "", title: "", date: context.selected(), time: null, description: "", kind, annual: kind === "birthday", reminders: kind === "birthday" ? [1440, 0] : kind === "reminder" ? [0] : [15], notifyAt: "09:00" };
    dialog = document.createElement("dialog");
    dialog.className = "event-dialog";
    dialog.innerHTML = `<form id="event-form">
      <header><h2>${existing ? "Editar" : "Agregar"} ${kindLabels[kind].toLowerCase()}</h2><button type="button" data-editor-close aria-label="Cerrar formulario">×</button></header>
      <label>${kind === "birthday" ? "Nombre" : "Título"}<input name="title" required maxlength="160" value="${escapeHtml(event.title)}" autocomplete="off" autofocus placeholder="${kind === "birthday" ? "Nombre de la persona" : "¿Qué tienes planeado?"}"></label>
      <div class="form-columns"><label>Fecha<input name="date" type="date" required min="1900-01-01" max="9999-12-31" value="${event.date}"></label>${kind === "birthday" ? "" : `<label>Hora<input name="time" type="time" value="${event.time ?? ""}" ${event.time ? "" : "disabled"}></label>`}</div>
      ${kind === "birthday" ? '<p class="form-note">Se repite cada año. Los cumpleaños del 29 de febrero se muestran el 28 en años no bisiestos.</p>' : `<label class="inline-check"><input type="checkbox" name="allDay" ${event.time ? "" : "checked"}> Todo el día</label>
      <details class="event-additional" ${event.weekly || event.annual ? "open" : ""}><summary>Configuraciones adicionales</summary>
        <label>Repetición<select name="repeat"><option value="none" ${!event.weekly && !event.annual ? "selected" : ""}>No repetir</option><option value="weekly" ${event.weekly ? "selected" : ""}>Cada semana, los días elegidos</option><option value="annual" ${event.annual ? "selected" : ""}>Cada año</option></select></label>
        <div data-weekly ${event.weekly ? "" : "hidden"}>
          <fieldset class="weekly-days"><legend>Días de la semana</legend>${["Lunes", "Martes", "Miércoles", "Jueves", "Viernes", "Sábado", "Domingo"].map((day, index) => `<label class="inline-check"><input type="checkbox" name="weekdays" value="${index + 1}" ${(event.weekly?.days ?? [((fromKey(event.date).getDay() + 6) % 7) + 1]).includes(index + 1) ? "checked" : ""}>${day}</label>`).join("")}</fieldset>
          <div class="repeat-presets"><button type="button" data-repeat-days="1,2,3,4,5">Lunes a viernes</button><button type="button" data-repeat-days="1,2,3,4,5,6,7">Todos los días</button></div>
          <label>Finaliza<select name="repeatEnd"><option value="never" ${!event.weekly?.until ? "selected" : ""}>Sin fecha de fin</option><option value="date" ${event.weekly?.until ? "selected" : ""}>En una fecha</option></select></label>
          <label data-repeat-until ${event.weekly?.until ? "" : "hidden"}>Último día de la serie<input type="date" name="repeatUntil" min="${event.date}" max="9999-12-31" value="${event.weekly?.until ?? ""}"></label>
          <p class="form-note">La fecha de arriba es el inicio de la serie. Sin fecha de fin, continúa hasta que la cambies o elimines. Para detener futuras reuniones conservando las anteriores, elige un último día.</p>
        </div>
        ${existing && (event.weekly || event.annual) ? '<p class="form-note">Estás editando toda la serie. Los cambios se aplican a todas sus fechas.</p>' : ""}
      </details>`}
      <label>Descripción <span class="optional">(opcional)</span><textarea name="description" maxlength="4000" rows="2">${escapeHtml(event.description)}</textarea></label>
      <fieldset class="reminder-options"><legend>Avisos de Windows</legend>
      ${((kind === "birthday" ? [[4320,"3 días antes"],[2880,"2 días antes"],[1440,"1 día antes"],[0,"El mismo día"]] : [[0,"A la hora"],[5,"5 min antes"],[15,"15 min antes"],[30,"30 min antes"],[60,"1 h antes"],[120,"2 h antes"],[1440,"1 día antes"]]) as [number,string][]).map(([minutes,label]) => `<label class="inline-check"><input name="reminders" type="checkbox" value="${minutes}" ${event.reminders.includes(minutes) ? "checked" : ""}> ${label}</label>`).join("")}
      </fieldset>
      <label class="day-notify-time" ${event.time ? "hidden" : ""}>Hora de las notificaciones para eventos de todo el día<input name="notifyAt" type="time" required value="${event.notifyAt}"></label>
      <p class="form-note">Los avisos funcionan mientras Calendario siga en la bandeja. Sin opciones marcadas, no se enviarán avisos.</p>
      <p class="form-error" role="alert" hidden></p>
      <footer>${existing ? `<button type="button" class="delete-event" data-delete>${event.weekly || event.annual ? "Eliminar serie" : "Eliminar"}</button>` : ""}<span></span><button type="button" data-editor-close>Cancelar</button><button type="submit" class="primary">Guardar</button></footer>
    </form>`;
    document.body.append(dialog);
    const current = dialog;
    const form = current.querySelector<HTMLFormElement>("form")!;
    const error = current.querySelector<HTMLElement>(".form-error")!;
    let confirmDelete = false;
    const setBusy = (value: boolean): void => {
      busy = value;
      current.querySelectorAll<HTMLButtonElement>("button").forEach(button => button.disabled = value);
    };
    const close = (): void => { if (!busy) current.close(); };
    current.querySelectorAll("[data-editor-close]").forEach(button => button.addEventListener("click", close));
    current.addEventListener("cancel", e => { if (busy) e.preventDefault(); });
    current.addEventListener("close", () => {
      current.remove(); if (dialog === current) dialog = undefined;
      void nativeCommand("set_editor_open", { open: false }).catch(context.error);
      window.dispatchEvent(new Event("calendar-overlay-closed"));
      document.querySelector<HTMLElement>(".day.selected")?.focus();
    });
    form.querySelector<HTMLInputElement>('[name="allDay"]')?.addEventListener("change", () => {
      const allDay = form.elements.namedItem("allDay") as HTMLInputElement;
      (form.elements.namedItem("time") as HTMLInputElement).disabled = allDay.checked;
      current.querySelector<HTMLElement>(".day-notify-time")!.hidden = !allDay.checked;
    });
    const report = (cause: unknown): void => { error.hidden = false; error.textContent = String(cause); };
    function updateRepeatControls(): void {
      const weekly = (form.elements.namedItem("repeat") as HTMLSelectElement | null)?.value === "weekly";
      const ends = (form.elements.namedItem("repeatEnd") as HTMLSelectElement | null)?.value === "date";
      const section = form.querySelector<HTMLElement>("[data-weekly]");
      if (section) section.hidden = !weekly;
      const untilLabel = form.querySelector<HTMLElement>("[data-repeat-until]");
      if (untilLabel) untilLabel.hidden = !ends;
      const until = form.elements.namedItem("repeatUntil") as HTMLInputElement | null;
      if (until) { until.disabled = !weekly || !ends; until.required = weekly && ends; until.min = (form.elements.namedItem("date") as HTMLInputElement).value; }
    }
    form.addEventListener("change", updateRepeatControls);
    form.querySelectorAll<HTMLButtonElement>("[data-repeat-days]").forEach(button => button.addEventListener("click", () => {
      const days = button.dataset.repeatDays!.split(",");
      form.querySelectorAll<HTMLInputElement>('[name="weekdays"]').forEach(input => { input.checked = days.includes(input.value); });
    }));
    updateRepeatControls();
    form.addEventListener("submit", e => {
      e.preventDefault();
      if (busy || !form.reportValidity()) return;
      const values = new FormData(form);
      const allDay = kind === "birthday" || values.get("allDay") === "on";
      const time = String(values.get("time") ?? "");
      if (!allDay && !time) { report("Indica una hora o marca «Todo el día»."); return; }
      const weekly = values.get("repeat") === "weekly";
      const days = values.getAll("weekdays").map(Number);
      const until = weekly && values.get("repeatEnd") === "date" ? String(values.get("repeatUntil") ?? "") : null;
      if (weekly && !days.length) { report("Selecciona al menos un día de la semana."); return; }
      if (until !== null && (!until || until < String(values.get("date")))) { report("El último día no puede ser anterior al inicio de la serie."); return; }
      setBusy(true); error.hidden = true;
      void saveEvent({ ...event, title: String(values.get("title") ?? ""), date: String(values.get("date") ?? ""),
        time: allDay ? null : time, description: String(values.get("description") ?? ""),
        annual: kind === "birthday" || values.get("repeat") === "annual",
        weekly: weekly ? { days, until } : null,
        reminders: values.getAll("reminders").map(Number), notifyAt: String(values.get("notifyAt") ?? "09:00") })
        .then(saved => { setBusy(false); current.close(); context.select(saved.date); context.changed(); })
        .catch(cause => { setBusy(false); report(cause); });
    });
    current.querySelector<HTMLButtonElement>("[data-delete]")?.addEventListener("click", e => {
      if (busy) return;
      const button = e.currentTarget as HTMLButtonElement;
      if (!confirmDelete) { confirmDelete = true; button.textContent = event.weekly || event.annual ? "¿Eliminar toda la serie?" : "¿Eliminar evento?"; report(event.weekly || event.annual ? "Se eliminarán todas las fechas de esta serie y sus avisos. Pulsa de nuevo para confirmar." : "Pulsa de nuevo para confirmar la eliminación."); return; }
      setBusy(true);
      void deleteEvent(event.id).then(() => { setBusy(false); current.close(); context.changed(); })
        .catch(cause => { setBusy(false); report(cause); });
    });
    current.showModal();
  }
  document.addEventListener("contextmenu", e => {
    const target = e.target as Element;
    // Keep text editing menus; replace WebView navigation menus on app surfaces.
    if (target.closest("input,textarea,select")) return;
    e.preventDefault();
    if (document.querySelector(".next-widget")) return;
    if (dialog?.open || target.closest(".settings-panel")) { closeMenu(); return; }
    const date = target.closest<HTMLElement>("[data-date]")?.dataset.date ?? context.selected();
    context.select(date);
    closeMenu();
    menu = document.createElement("div"); menu.className = "calendar-menu"; menu.setAttribute("role", "menu");
    menu.setAttribute("aria-label", "Agregar a la agenda");
    menu.innerHTML = `<p>${fullDateLabel.format(fromKey(date))}</p><button role="menuitem" data-kind="event">Agregar evento</button><button role="menuitem" data-kind="birthday">Agregar cumpleaños</button><button role="menuitem" data-kind="reminder">Agregar recordatorio</button>`;
    document.body.append(menu);
    const bounds = menu.getBoundingClientRect();
    menu.style.left = Math.max(8, Math.min(e.clientX, innerWidth - bounds.width - 8)) + "px";
    menu.style.top = Math.max(8, Math.min(e.clientY, innerHeight - bounds.height - 8)) + "px";
    menu.querySelectorAll<HTMLButtonElement>("button").forEach(button => button.addEventListener("click", () => {
      void openEditor(button.dataset.kind as EventKind).catch(context.error);
    }));
    menu.querySelector<HTMLButtonElement>("button")?.focus();
  });
  document.addEventListener("pointerdown", e => { if (menu && !menu.contains(e.target as Node)) closeMenu(); });
  document.addEventListener("keydown", e => {
    if (!menu) return;
    if (e.key === "Escape") { e.preventDefault(); e.stopImmediatePropagation(); closeMenu(); document.querySelector<HTMLElement>(".day.selected")?.focus(); }
    else if (["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) {
      e.preventDefault(); e.stopImmediatePropagation();
      const buttons = Array.from(menu.querySelectorAll<HTMLButtonElement>("button"));
      const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
      const next = e.key === "Home" ? 0 : e.key === "End" ? buttons.length - 1 : (index + (e.key === "ArrowDown" ? 1 : -1) + buttons.length) % buttons.length;
      buttons[next]?.focus();
    }
  }, true);
  document.addEventListener("click", e => {
    const target = (e.target as Element).closest<HTMLElement>("[data-event-id],[data-add-event]");
    if (!target) return;
    const id = target.dataset.eventId;
    const existing = id ? context.find(id) : undefined;
    if (id && !existing) return;
    void openEditor(existing?.kind ?? "event", existing).catch(context.error);
  });
  window.addEventListener("blur", closeMenu);
  return { open: (event: CalendarEvent) => openEditor(event.kind, event) };
}

