import { useState } from "react";
import type { FormEvent, ReactNode } from "react";
import { errorText } from "./api";

export function field(data: FormData, name: string): string {
  const value = data.get(name);
  return typeof value === "string" ? value : "";
}

export function Notice({ message }: Readonly<{ message: string | null }>) {
  return message ? <p className="notice" role="alert">{message}</p> : null;
}

type ActionFormProps = Readonly<{
  children?: ReactNode;
  label: string;
  perform: (data: FormData) => Promise<void>;
  reset?: boolean;
  secondary?: boolean;
}>;

export function ActionForm({ children, label, perform, reset = true, secondary = false }: ActionFormProps) {
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const form = event.currentTarget;
    setBusy(true);
    setError(null);
    try {
      await perform(new FormData(form));
      if (reset) form.reset();
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  }

  return (
    <form onSubmit={submit}>
      <fieldset disabled={busy} className="form-stack">
        {children}
        <button type="submit" className={secondary ? "secondary" : "primary"}>{busy ? "Memproses…" : label}</button>
      </fieldset>
      <Notice message={error} />
    </form>
  );
}
