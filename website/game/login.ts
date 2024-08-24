import { BackendConnection } from "./backend";
import { Io } from "./io";

export async function login({
  io,
  abort,
}: {
  io: Io;
  abort: AbortSignal;
}): Promise<BackendConnection> {
  const username = await io.prompt({ abort, text: "Username:" });
  const password = await io.prompt({ abort, text: "Password:", hidden: true });
  const connection = await BackendConnection.connect(abort, {
    username,
    password,
  });
  return connection;
}
