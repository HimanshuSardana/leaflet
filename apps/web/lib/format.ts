const UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

/** `12.4 MB` — one decimal past the first unit, because 1.4 GB matters. */
export function formatBytes(bytes: number): string {
  let value = Math.max(0, bytes);
  let unit = 0;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = unit === 0 || value >= 100 ? 0 : 1;
  return `${value.toFixed(digits)} ${UNITS[unit]}`;
}

const STEPS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["second", 60],
  ["minute", 60],
  ["hour", 24],
  ["day", 7],
  ["week", 4.35],
  ["month", 12],
  ["year", Number.POSITIVE_INFINITY],
];

/** `3 minutes ago` — relative for the last day, a date after that. */
export function formatAdded(iso: string, now = Date.now()): string {
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return "";

  let delta = (then - now) / 1000;
  if (Math.abs(delta) < 45) return "just now";

  for (const [unit, span] of STEPS) {
    if (Math.abs(delta) < span) {
      return new Intl.RelativeTimeFormat(undefined, { numeric: "auto" }).format(
        Math.round(delta),
        unit,
      );
    }
    delta /= span;
  }
  return new Date(then).toLocaleDateString();
}
