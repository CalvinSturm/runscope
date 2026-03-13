import type { MetricRecord } from "./types";

export const EMPTY_TOKEN = "—";

export function formatDateTime(value: string | null | undefined): string {
  if (!value) {
    return EMPTY_TOKEN;
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
    return EMPTY_TOKEN;
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
  return metric.value_text ?? EMPTY_TOKEN;
}

export function formatNumericDelta(value: number | null | undefined, unit?: string | null): string {
  if (value == null) {
    return EMPTY_TOKEN;
  }
  const prefix = value > 0 ? "+" : "";
  const rounded = Math.abs(value) >= 100 ? value.toFixed(0) : value.toFixed(2);
  return unit ? `${prefix}${rounded} ${unit}` : `${prefix}${rounded}`;
}

export function formatPercentDelta(value: number | null | undefined): string {
  if (value == null) {
    return EMPTY_TOKEN;
  }
  const prefix = value > 0 ? "+" : "";
  return `${prefix}${value.toFixed(2)}%`;
}

export function formatOptionalText(value: string | null | undefined): string {
  if (!value || value.trim().length === 0) {
    return EMPTY_TOKEN;
  }
  return value;
}

export function formatRelativeAge(value: string | null | undefined): string {
  if (!value) {
    return EMPTY_TOKEN;
  }
  const timestamp = new Date(value).getTime();
  if (Number.isNaN(timestamp)) {
    return EMPTY_TOKEN;
  }
  const deltaMs = Date.now() - timestamp;
  const deltaMinutes = Math.max(0, Math.floor(deltaMs / 60000));
  if (deltaMinutes < 1) {
    return "just now";
  }
  if (deltaMinutes < 60) {
    return `${deltaMinutes}m ago`;
  }
  const deltaHours = Math.floor(deltaMinutes / 60);
  if (deltaHours < 24) {
    return `${deltaHours}h ago`;
  }
  const deltaDays = Math.floor(deltaHours / 24);
  return `${deltaDays}d ago`;
}

export function compactMetricLabel(key: string): string {
  return key
    .replace(/^ux\.[^.]+\./, "")
    .replace(/^ux\./, "")
    .replace(/^latency_/, "lat ")
    .replace(/^validation_/, "val ")
    .replace(/^completion_/, "comp ")
    .replace(/^task_/, "task ")
    .replace(/_rate$/i, "")
    .replace(/_score$/i, " score")
    .replace(/_/g, " ")
    .trim();
}
