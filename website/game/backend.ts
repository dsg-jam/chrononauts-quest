import { safeJsonParse } from "@/utils/json";

const BACKEND_URL = process.env.NEXT_PUBLIC_BACKEND_URL ?? "";

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
  private updateListeners: Array<() => void>;
  private level: Level | null;

  constructor(ws: WebSocket) {
    ws.binaryType = "arraybuffer";
    this.ws = ws;
    this.updateListeners = [];
    this.level = null;

    ws.addEventListener("open", (event) => {
      console.info("open", event);
    });
    ws.addEventListener("close", (event) => {
      console.info("close", event);
    });
    ws.addEventListener("message", (event) => {
      let data = event.data as string | ArrayBuffer;
      let msg;
      try {
        msg = safeJsonParse(data) as Msg;
      } catch (err) {
        console.error("error parsing message", { event, err });
        return;
      }

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

  getLevel(): Level {
    return this.level ?? "L0";
  }

  private setLevel(level: Level): void {
    if (this.level === level) {
      return;
    }
    this.level = level;
    this.dispatchUpdate();
  }

  private dispatchUpdate(): void {
    for (const listener of this.updateListeners) {
      try {
        listener();
      } catch (err) {
        console.error("error in update listener", err);
      }
    }
  }

  private onUpdate(listener: () => void): () => void {
    this.updateListeners.push(listener);
    return () => {
      const index = this.updateListeners.indexOf(listener);
      if (index !== -1) {
        this.updateListeners.splice(index, 1);
      }
    };
  }

  private async waitForConnection(abort: AbortSignal): Promise<void> {
    let unsubscribe = null as (() => void) | null;
    const promise = new Promise<void>((resolve, reject) => {
      const onUpdate = () => {
        if (this.level !== null) {
          resolve();
        }
      };
      const onClose = () => {
        reject(new Error("Connection closed"));
      };
      const onError = () => {
        reject(new Error("Connection error"));
      };
      const onAbort = () => {
        reject(abort.reason);
      };

      const unsubscribeUpdate = this.onUpdate(onUpdate);
      this.ws.addEventListener("close", onClose);
      this.ws.addEventListener("error", onError);
      abort.addEventListener("abort", onAbort);

      unsubscribe = () => {
        unsubscribeUpdate();
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
