import { useEffect, useState } from "react";
import { ApiError, errorText, read } from "./api";
import type { Decoder } from "./api";

type Load<T> = Readonly<{ data: T | null; error: string | null }>;

export function useResource<T>(path: string, schema: Decoder<T>, token: string, revision: number, poll = false): Load<T> {
  const [resource, setResource] = useState<Load<T>>({ data: null, error: null });
  useEffect(() => {
    const controller = new AbortController();
    let timer: number | undefined;

    async function refresh() {
      try {
        const data = await read(path, schema, { token, signal: controller.signal });
        if (!controller.signal.aborted) setResource({ data, error: null });
      } catch (error) {
        if (!controller.signal.aborted) {
          const accessDenied = error instanceof ApiError && [401, 403, 404].includes(error.status);
          setResource((previous) => ({ data: accessDenied ? null : previous.data, error: errorText(error) }));
        }
      }
      if (poll && !controller.signal.aborted) timer = window.setTimeout(refresh, 5000);
    }

    void refresh();
    return () => {
      controller.abort();
      window.clearTimeout(timer);
    };
  }, [path, schema, token, revision, poll]);
  return resource;
}
