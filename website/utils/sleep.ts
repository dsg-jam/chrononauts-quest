export async function sleep(ms: number, abort: AbortSignal) {
  let unsubscribe = null as (() => void) | null;
  const promise = new Promise<void>((resolve, reject) => {
    const onAbort = () => {
      reject(abort.reason);
    };

    const id = setTimeout(resolve, ms);
    abort.addEventListener("abort", onAbort);

    unsubscribe = () => {
      clearTimeout(id);
      abort.removeEventListener("abort", onAbort);
    };
  });

  try {
    await promise;
  } finally {
    if (unsubscribe) {
      unsubscribe();
    }
  }
}
