import { boot, login } from "./login";
import { Terminal } from "@/components/terminal";

export async function run(terminal: Terminal) {
  await boot(terminal);
  const backend = await login(terminal);
  while (!terminal.abort.aborted) {
    await terminal.input();
  }
}
