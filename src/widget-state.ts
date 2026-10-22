// Serialize native resize requests while retaining only the latest hover intent.
export function expansionState(resize: (expanded: boolean) => Promise<void>, paint: (expanded: boolean) => void, error: (cause: unknown) => void) {
  let desired = false, applied = false, running = false, paused = false, dirty = false;
  async function reconcile(): Promise<void> {
    if (running || paused) return;
    running = true;
    try {
      while (!paused && (dirty || applied !== desired)) {
        dirty = false;
        const next = desired;
        paint(next);
        await resize(next);
        applied = next;
      }
    } catch (cause) {
      desired = applied = false; paint(false);
      try { await resize(false); } catch { /* original error is more useful */ }
      error(cause);
    } finally { running = false; }
  }
  return {
    request(next: boolean, measure = false) { desired = next; dirty ||= measure; void reconcile(); },
    pause(active: boolean) { paused = active; if (!active) { desired = false; void reconcile(); } },
    desired: () => desired,
  };
}
