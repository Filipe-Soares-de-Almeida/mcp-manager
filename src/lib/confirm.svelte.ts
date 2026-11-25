export interface ConfirmOptions {
  title: string;
  text: string;
  confirm: string;
  danger?: boolean;
}

class Confirmer {
  open = $state(false);
  opts = $state<ConfirmOptions>({ title: "", text: "", confirm: "Confirmar" });
  #resolve: ((ok: boolean) => void) | null = null;

  ask(opts: ConfirmOptions): Promise<boolean> {
    this.#resolve?.(false);
    this.opts = opts;
    this.open = true;
    return new Promise((resolve) => (this.#resolve = resolve));
  }

  answer(ok: boolean) {
    this.open = false;
    this.#resolve?.(ok);
    this.#resolve = null;
  }
}

export const confirmer = new Confirmer();
