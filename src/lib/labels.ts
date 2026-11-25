import { i18n } from "./i18n.svelte";
import type { OpKind } from "./types";

export function kindLabel(kind: OpKind): string {
  return i18n.t.kind[kind];
}

export function describe(kind: OpKind | "", names: string[], fallback: string): string {
  const d = i18n.t.describe;
  const [a, b] = names;
  switch (kind) {
    case "add":
      return d.add(a);
    case "update":
      return d.update(a);
    case "rename":
      return d.rename(a, b);
    case "remove":
      return d.remove(a);
    case "disable":
      return d.disable(a);
    case "enable":
      return d.enable(a);
    case "restore":
      return d.restore(names.length);
    default:
      return fallback;
  }
}

export function sourceLabel(source: string): string {
  return i18n.t.source[source] ?? source;
}

export function reasonLabel(reason: string): string {
  return i18n.t.reason[reason] ?? reason;
}

export function when(iso: string): string {
  const d = new Date(iso.includes("T") ? iso : iso.replace(" ", "T"));
  if (Number.isNaN(d.getTime())) return iso;
  const tag = i18n.locale;
  const today = new Date();
  const time = d.toLocaleTimeString(tag, { hour: "2-digit", minute: "2-digit" });
  if (d.toDateString() === today.toDateString()) return i18n.t.when.today(time);
  const yesterday = new Date(today.getTime() - 86_400_000);
  if (d.toDateString() === yesterday.toDateString()) return i18n.t.when.yesterday(time);
  return `${d.toLocaleDateString(tag, { day: "2-digit", month: "short", year: "numeric" })}, ${time}`;
}

export function kilobytes(size: number): string {
  return `${(size / 1024).toLocaleString(i18n.locale, { maximumFractionDigits: 1 })} KB`;
}
