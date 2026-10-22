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
    if (!dragging) timer = setTimeout(() => request(true), 100);
  }
  root.addEventListener("pointerenter", enter);
  root.addEventListener("pointermove", () => { if (!inside) enter(); });
  root.addEventListener("pointerleave", () => { inside = false; leave(); });
  window.addEventListener("calendar-overlay-closed", () => { if (!inside) leave(); });
  window.addEventListener("blur", () => { inside = false; if (!overlayOpen()) leave(); });
  root.addEventListener("keydown", event => {
    if (event.key === "Escape") request(false);
    else if (event.key === "Enter" && (event.target as HTMLElement).matches("[data-mini-toggle]")) request(!state.desired());
  });
  return {
    reset: () => { clearTimeout(timer); state.request(enabled() && inside, true); },
    refresh: () => state.request(state.desired() && enabled(), true),
    drag: (active: boolean) => {
      dragging = active; state.pause(active); clearTimeout(timer);
      if (!active) { inside = false; request(false); }
    },
  };
}
