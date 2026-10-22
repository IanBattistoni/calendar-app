import { expansionState } from "./widget-state.ts";
import { nativeCommand } from "./preferences";
import { editorIsOpen } from "./event-editor";
export { remainingTime, upcomingRows, miniEvent } from "./widget-data";

export function installWidgetHover(root: HTMLElement, enabled: () => boolean, changed: (expanded: boolean) => void, error: (cause: unknown) => void) {
  let dragging = false, inside = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const overlayOpen = () => editorIsOpen() || !!document.querySelector(".calendar-menu");
  const state = expansionState(async next => {
    await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
    const panel = root.querySelector<HTMLElement>(".next-widget");
    const summary = panel?.querySelector<HTMLElement>(".mini-summary");
    if (!panel || !summary) return;
    const shell = getComputedStyle(root), border = getComputedStyle(panel);
    const collapsedHeight = summary.getBoundingClientRect().height +
      parseFloat(shell.paddingTop) + parseFloat(shell.paddingBottom) +
      parseFloat(border.borderTopWidth) + parseFloat(border.borderBottomWidth);
    await nativeCommand("set_widget_expanded", { expanded: next, metrics: {
      collapsedHeight, expandedHeight: collapsedHeight + 280, viewportHeight: innerHeight,
    } });
  }, changed, error);
  function request(next: boolean): void { state.request(next && enabled()); }
  function leave(): void {
    clearTimeout(timer);
    timer = setTimeout(() => { if (!inside && !overlayOpen()) request(false); }, 160);
  }
  function enter(): void {
    inside = true; clearTimeout(timer);
    if (!dragging && !moveStart) timer = setTimeout(() => request(true), 100);
  }
  let moveStart: { x: number; y: number; pointerId: number } | undefined;
  let suppressClick = false;
  root.addEventListener("pointerdown", event => {
    if (event.button !== 0 || overlayOpen() || !(event.target as Element).closest(".widget-panel")) return;
    if ((event.target as Element).closest("input, textarea, select, a, [contenteditable=true]")) return;
    // Leave the native scrollbar available for scrolling the event list.
    const scroller = (event.target as Element).closest<HTMLElement>(".upcoming-scroll, .widget-agenda");
    if (scroller && scroller.scrollHeight > scroller.clientHeight &&
      event.clientX >= scroller.getBoundingClientRect().right - 14) return;
    suppressClick = false;
    moveStart = { x: event.screenX, y: event.screenY, pointerId: event.pointerId };
    clearTimeout(timer);
  });
  window.addEventListener("pointermove", event => {
    if (!moveStart || event.pointerId !== moveStart.pointerId) return;
    if (!(event.buttons & 1)) { moveStart = undefined; return; }
    if (Math.hypot(event.screenX - moveStart.x, event.screenY - moveStart.y) < 5) return;
    moveStart = undefined;
    suppressClick = true;
    event.preventDefault();
    dragging = true; state.pause(true); clearTimeout(timer);
    void nativeCommand("start_widget_drag").catch(cause => {
      dragging = false; state.pause(false); suppressClick = false; error(cause);
    });
  });
  window.addEventListener("pointerup", () => { moveStart = undefined; });
  window.addEventListener("pointercancel", () => { moveStart = undefined; });
  root.addEventListener("click", event => {
    if (suppressClick) { event.preventDefault(); event.stopImmediatePropagation(); suppressClick = false; }
  }, true);
  root.addEventListener("pointerenter", event => { if (!(event.target as HTMLElement).closest(".widget-resize")) enter(); });
  root.addEventListener("pointermove", () => { if (!inside) enter(); });
  root.addEventListener("pointerleave", () => { inside = false; leave(); });
  window.addEventListener("calendar-overlay-closed", () => { if (!inside) leave(); });
  window.addEventListener("blur", () => { inside = false; if (!overlayOpen()) leave(); });
  root.addEventListener("keydown", event => {
    if (event.key === "Escape") request(false);
    else if (event.key === "Enter" && (event.target as HTMLElement).matches("[data-mini-toggle]")) request(!state.desired());
  });
  for (const direction of ["East", "West"]) {
    const handle = document.createElement("div");
    handle.className = `widget-resize resize-${direction}`;
    handle.title = "Arrastrar para ajustar el ancho";
    handle.addEventListener("pointerenter", () => { clearTimeout(timer); });
    handle.addEventListener("pointerdown", event => {
      if (event.button !== 0) return;
      event.preventDefault(); event.stopPropagation();
      dragging = true; state.pause(true); clearTimeout(timer);
      void nativeCommand("start_widget_resize", { direction }).catch(cause => {
        dragging = false; state.pause(false); error(cause);
      });
    });
    document.body.append(handle);
  }
  // Keep generous move grips separate from the side resize grips and content.
  for (const edge of ["top", "bottom"]) {
    const handle = document.createElement("div");
    handle.className = `widget-move move-${edge}`;
    handle.title = "Arrastrar para mover el widget";
    handle.addEventListener("pointerenter", () => { inside = true; clearTimeout(timer); });
    handle.addEventListener("pointerleave", () => { inside = false; leave(); });
    handle.addEventListener("pointerdown", event => {
      if (event.button !== 0) return;
      event.preventDefault(); event.stopPropagation();
      dragging = true; state.pause(true); clearTimeout(timer);
      void nativeCommand("start_widget_drag").catch(cause => {
        dragging = false; state.pause(false); error(cause);
      });
    });
    document.body.append(handle);
  }
  return {
    reset: () => { clearTimeout(timer); state.request(enabled() && inside, true); },
    refresh: () => state.request(state.desired() && enabled(), true),
    drag: (active: boolean) => {
      moveStart = undefined;
      dragging = active; state.pause(active); clearTimeout(timer);
      if (!active) { inside = false; request(false); }
    },
  };
}
