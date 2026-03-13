import type { MetricRecord } from "./types";

export function formatDateTime(value: string | null | undefined): string {
  if (!value) {
    return "n/a";
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(date);
}

export function formatDuration(value: number | null | undefined): string {
  if (value == null) {
    return "n/a";
  }
  if (value < 1000) {
    return `${value} ms`;
  }
  const seconds = value / 1000;
  if (seconds < 60) {
    return `${seconds.toFixed(1)} s`;
  }
  const minutes = Math.floor(seconds / 60);
  const remainder = seconds % 60;
  return `${minutes}m ${remainder.toFixed(0)}s`;
}

export function formatMetric(metric: MetricRecord): string {
  if (metric.value_num != null) {
    const rounded =
      Math.abs(metric.value_num) >= 100
        ? metric.value_num.toFixed(0)
        : metric.value_num.toFixed(2);
    return metric.unit ? `${rounded} ${metric.unit}` : rounded;
  }
  return metric.value_text ?? "n/a";
}
