import { allCommands } from "./commands";
import { boot, login } from "./login";
import { Vfs } from "./vfs";
import { Terminal } from "@/components/terminal";

// Set to true to skip the boot animation and login process during development.
const skipBootAnimation = false;

export async function run(terminal: Terminal) {
  if (!skipBootAnimation) {
    await boot(terminal);
  }

  const backend = await login(terminal);

  if (!skipBootAnimation) {
    await terminal.type(
      ["Login successful.", "", "", "File system decrypted.", "", "", ""],
      {
        lineEndDelay: 250,
      },
    );
    await terminal.typeLine("Activating shell.......", { endDelay: 5000 });
  }
  terminal.clear();

  const vfs = new Vfs();

  while (!terminal.abort.aborted) {
    const text = await terminal.input();
    const [commandName, ...args] = text.split(" ");
    if (commandName === "") {
      continue;
    }

    const command = allCommands.find((c) => c.name === commandName);
    if (!command) {
      await terminal.type(
        `Command not found: ${commandName}. Use 'help' for a list of commands.`,
      );
      continue;
    }
    await command.execute({ terminal, backend, vfs, args });
  }
}
