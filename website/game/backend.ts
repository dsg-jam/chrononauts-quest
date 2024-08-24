import { safeJsonParse } from "@/utils/json";

const BACKEND_URL = process.env.BACKEND_URL ?? "";

export type Level = "L0" | "L1" | "L2" | "L3" | "L4";

export interface GameState {
  level: Level;
}

export interface LoginCredentials {
  username: string;
  password: string;
}

type Msg = {
  "@type": "GAME_STATE";
} & MsgGameState;

type MsgGameState = {
  level: Level;
};

export class BackendConnection {
  private ws: WebSocket;
  private level: Level;

  constructor(ws: WebSocket) {
    this.ws = ws;
    this.level = "L0";

    ws.addEventListener("open", (event) => {
      console.info("open", event);
    });
    ws.addEventListener("close", (event) => {
      console.info("close", event);
    });
    ws.addEventListener("message", (event) => {
      console.info("message", event);
      const msg = safeJsonParse(event.data) as Msg;
      switch (msg["@type"]) {
        case "GAME_STATE":
          this.setLevel(msg.level);
          break;
      }
    });
    ws.addEventListener("error", (event) => {
      console.error("error", event);
    });
  }

  static async connect(
    abort: AbortSignal,
    login: LoginCredentials,
  ): Promise<BackendConnection> {
    const connection = new BackendConnection(createWebSocket(login));
    await connection.waitForConnection(abort);
    return connection;
  }

  getLevel(): Level | null {
    return this.level;
  }

  private setLevel(level: Level): void {
    this.level = level;
  }

  private async waitForConnection(abort: AbortSignal): Promise<void> {
    let unsubscribe = null as (() => void) | null;
    const promise = new Promise<void>((resolve, reject) => {
      const onOpen = () => {
        resolve();
      };
      const onClose = () => {
        reject(new Error("Connection closed"));
      };
      const onError = () => {
        reject(new Error("Connection error"));
      };
      const onAbort = (reason: any) => {
        reject(reason);
      };

      this.ws.addEventListener("open", onOpen);
      this.ws.addEventListener("close", onClose);
      this.ws.addEventListener("error", onError);
      abort.addEventListener("abort", onAbort);

      unsubscribe = () => {
        this.ws.removeEventListener("open", onOpen);
        this.ws.removeEventListener("close", onClose);
        this.ws.removeEventListener("error", onError);
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
}

function createWebSocket(login: LoginCredentials): WebSocket {
  const url = new URL(BACKEND_URL, window.location.href);
  url.searchParams.set("username", login.username);
  url.searchParams.set("password", login.password);
  return new WebSocket(url);
}
