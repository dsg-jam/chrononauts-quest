import { JSONCompatible, safeJsonParse, safeJsonStringify } from "@/utils/json";
import { useCallback, useSyncExternalStore } from "react";

export type SetState<T> = (value: T | undefined) => void;

export function useLocalStorage<T extends JSONCompatible<T>>(
  key: string,
  defaultValue: T,
): [T, SetState<T>] {
  const store = useSyncExternalStore(
    useLocalStorageSubscribe,
    () => window.localStorage.getItem(key),
    () => null,
  );
  const setState = useCallback(
    (value: T | undefined) => {
      if (value === undefined) {
        window.localStorage.removeItem(key);
      } else {
        window.localStorage.setItem(key, safeJsonStringify(value));
      }
    },
    [key],
  );

  const value = store ? (safeJsonParse(store) as T) : defaultValue;
  return [value, setState];
}

function useLocalStorageSubscribe(callback: () => void) {
  window.addEventListener("storage", callback);
  return () => window.removeEventListener("storage", callback);
}
