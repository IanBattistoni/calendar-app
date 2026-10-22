import assert from "node:assert/strict";
import { test } from "node:test";
import { monthDays, dayKey, fromKey, shiftDay } from "../src/calendar.ts";

test("leap year contains February 29 once", () => {
  const days = monthDays(2028, 1, 1).map(dayKey);
  assert.equal(days.filter(d => d === "2028-02-29").length, 1);
  assert.ok(!monthDays(2027, 1, 1).map(dayKey).includes("2027-02-29"));
});
test("first weekday and year rollover", () => {
  for (const firstDay of [0, 1]) {
    const days = monthDays(2027, 0, firstDay);
    assert.equal(days.length, 42);
    assert.equal(days[0].getDay(), firstDay);
    assert.equal(new Set(days.map(dayKey)).size, 42);
    for (let i = 1; i < days.length; i++) assert.equal(dayKey(days[i]), dayKey(shiftDay(days[i - 1], 1)));
  }
});
test("civil dates round trip through daylight saving changes", () => {
  for (const date of ["2026-04-04", "2026-04-05", "2026-09-06", "2028-02-29", "2026-12-31"]) assert.equal(dayKey(fromKey(date)), date);
  assert.equal(dayKey(shiftDay(fromKey("2026-12-31"), 1)), "2027-01-01");
});

