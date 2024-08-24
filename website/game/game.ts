import { Io } from "./io";
import { login } from "./login";

export async function run({ abort, io }: { abort: AbortSignal; io: Io }) {
  const backend = await login({ abort, io });
}
