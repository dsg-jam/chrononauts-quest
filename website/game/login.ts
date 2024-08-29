import { BackendConnection } from "./backend";
import { Terminal } from "@/components/terminal";

export async function boot(terminal: Terminal): Promise<void> {
  terminal.clear();

  await terminal.type("Booting Chronix 0.1.0", { startDelay: 2000 });
  await terminal.type(
    [
      "...",
      "Detecting Terminal...",
      "...........",
      "Found wireless terminal.",
      ".",
      ".",
      ".",
      ".",
      ".",
    ],
    { lineEndDelay: 250 },
  );
  await terminal.type(["OK.", ""]);
}

export async function login(terminal: Terminal): Promise<BackendConnection> {
  while (true) {
    const username = await terminal.prompt("Username:");
    const password = await terminal.prompt("Password:", true);
    let connection;
    try {
      connection = await BackendConnection.connect(terminal.abort, {
        username,
        password,
      });
    } catch (err) {
      await terminal.prompt("Login failed. Please try again.");
      continue;
    }
    return connection;
  }
}
