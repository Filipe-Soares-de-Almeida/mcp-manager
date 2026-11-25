import { i18n } from "./i18n.svelte";
import type { Spec } from "./types";

export interface EnvRow {
  key: string;
  value: string;
}

export interface Form {
  name: string;
  command: string;
  args: string[];
  env: EnvRow[];
  /** Every other field of the entry, as JSON text. */
  extra: string;
}

const KNOWN = new Set(["command", "args", "env"]);

export function toForm(name: string, spec: Spec | null): Form {
  const s = spec ?? {};
  const extra: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(s)) if (!KNOWN.has(k)) extra[k] = v;
  return {
    name,
    command: typeof s.command === "string" ? s.command : "",
    args: Array.isArray(s.args) ? s.args.map(String) : [],
    env: s.env && typeof s.env === "object" ? Object.entries(s.env).map(([key, value]) => ({ key, value: String(value) })) : [],
    extra: Object.keys(extra).length ? JSON.stringify(extra, null, 2) : "",
  };
}

export type Errors = Partial<Record<"name" | "command" | "env" | "extra" | "paste", string>>;

export function validateName(name: string, taken: string[], original: string | null): string | null {
  const v = i18n.t.validate;
  if (!name.trim()) return v.nameEmpty;
  if (name !== name.trim()) return v.nameSpaces;
  if ([...name].length > 64) return v.nameLong;
  const bad = [...name].find((c) => !/[\p{L}\p{N}\-_. ]/u.test(c));
  if (bad) return v.nameChar(bad);
  if (name !== original && taken.includes(name)) return v.nameTaken;
  return null;
}

export function fromForm(form: Form, taken: string[], original: string | null): { spec: Spec | null; errors: Errors } {
  const v = i18n.t.validate;
  const errors: Errors = {};
  const nameError = validateName(form.name, taken, original);
  if (nameError) errors.name = nameError;
  if (!form.command.trim()) errors.command = v.commandEmpty;

  let extra: Record<string, unknown> = {};
  if (form.extra.trim()) {
    try {
      const parsed = JSON.parse(form.extra);
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) throw new Error();
      extra = parsed;
      const clash = Object.keys(extra).find((k) => KNOWN.has(k));
      if (clash) errors.extra = v.extraClash(clash);
    } catch {
      errors.extra = v.extraInvalid;
    }
  }

  const env: Record<string, string> = {};
  for (const row of form.env) {
    const key = row.key.trim();
    if (!key && !row.value) continue;
    if (!key) {
      errors.env = v.envNoName;
      continue;
    }
    if (key in env) errors.env = v.envDup(key);
    env[key] = row.value;
  }

  if (Object.keys(errors).length) return { spec: null, errors };
  const spec: Spec = { ...extra, command: form.command.trim() };
  const args = form.args.filter((a) => a !== "");
  if (args.length) spec.args = args;
  if (Object.keys(env).length) spec.env = env;
  return { spec, errors };
}

/** Reads what people paste from a README: a whole config, an mcpServers map, or one entry. */
export function parsePasted(text: string): { name: string | null; spec: Spec } {
  let value: unknown;
  try {
    value = JSON.parse(text.trim().replace(/,\s*$/, ""));
  } catch {
    // A bare `"name": { ... }` pair, as READMEs often show it.
    value = JSON.parse(`{${text.trim().replace(/,\s*$/, "")}}`);
  }
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("not an object");
  let obj = value as Record<string, unknown>;
  if (obj.mcpServers && typeof obj.mcpServers === "object") obj = obj.mcpServers as Record<string, unknown>;
  if (typeof obj.command === "string") return { name: null, spec: obj as Spec };
  const entries = Object.entries(obj).filter(([, v]) => v && typeof v === "object" && !Array.isArray(v));
  if (entries.length === 0) throw new Error("no server entry");
  const [name, spec] = entries[0];
  return { name, spec: spec as Spec };
}

function quote(arg: string): string {
  return /[\s"]/.test(arg) ? `"${arg.replace(/"/g, '\\"')}"` : arg;
}

export function commandLine(spec: Spec): string {
  const parts = [spec.command ?? "", ...(spec.args ?? [])];
  return parts.filter((p) => p !== "").map(quote).join(" ");
}
