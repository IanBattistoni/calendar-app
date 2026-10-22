// Civil dates stay in local time; never derive a day from toISOString().
export function dayKey(date: Date): string {
  return [date.getFullYear(), String(date.getMonth() + 1).padStart(2, "0"), String(date.getDate()).padStart(2, "0")].join("-");
}
export function fromKey(key: string): Date {
  const [year, month, day] = key.split("-").map(Number);
  return new Date(year, month - 1, day, 12);
}
export function shiftDay(date: Date, offset: number): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() + offset, 12);
}
export function monthDays(year: number, month: number, firstDay: number): Date[] {
  const first = new Date(year, month, 1, 12);
  const offset = (first.getDay() - firstDay + 7) % 7;
  return Array.from({ length: 42 }, (_, index) => shiftDay(first, index - offset));
}
export const monthLabel = new Intl.DateTimeFormat("es-CL", { month: "long", year: "numeric" });
export const fullDateLabel = new Intl.DateTimeFormat("es-CL", { weekday: "long", day: "numeric", month: "long", year: "numeric" });
export const dayLabel = new Intl.DateTimeFormat("es-CL", { weekday: "long", day: "numeric", month: "long" });
export const weekdayLabel = new Intl.DateTimeFormat("es-CL", { weekday: "short" });

