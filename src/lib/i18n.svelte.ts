import { enUS } from "./locales/en-US";
import { ptBR, type Dict } from "./locales/pt-BR";

export type Locale = "pt-BR" | "en-US";
export type LocaleSetting = "auto" | Locale;

function guess(): Locale {
  if (typeof navigator !== "undefined" && navigator.language.toLowerCase().startsWith("pt")) return "pt-BR";
  return "en-US";
}

/** The interface language. The Rust side resolves "auto" from the Windows
 *  display language and reports it in every snapshot; until the first one
 *  arrives, the browser's language stands in. */
class I18n {
  locale = $state<Locale>(guess());

  get t(): Dict {
    return this.locale === "pt-BR" ? ptBR : enUS;
  }

  set(locale: Locale) {
    this.locale = locale;
    if (typeof document !== "undefined") document.documentElement.lang = locale;
  }
}

export const i18n = new I18n();
