import { BackendConnection, LoginCredentials } from "./backend";
import { Terminal } from "@/components/terminal";
import { safeJsonParse, safeJsonStringify } from "@/utils/json";

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
  await terminal.type("OK.\n");
}

export async function login(terminal: Terminal): Promise<BackendConnection> {
  let firstAttempt = true;
  while (true) {
    const credentials = await getCredentials(terminal, firstAttempt);
    firstAttempt = false;

    let connection;
    try {
      connection = await BackendConnection.connect(terminal.abort, credentials);
    } catch (err) {
      await terminal.type("Login failed. Please try again.");
      continue;
    }
    return connection;
  }
}

async function getCredentials(
  terminal: Terminal,
  loadFromStorage: boolean,
): Promise<LoginCredentials> {
  if (loadFromStorage) {
    const raw = localStorage.getItem("login");
    if (raw) {
      await terminal.type("Loading cached credentials...\n");
      return safeJsonParse(raw) as LoginCredentials;
    }
  }

  const username = await terminal.prompt("Username:");
  const password = await terminal.prompt("Password:", true);
  const credentials = { username, password };

  localStorage.setItem("login", safeJsonStringify(credentials));
  return credentials;
}
