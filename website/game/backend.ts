import { safeJsonParse, safeJsonStringify } from "@/utils/json";

const backendUrl = process.env.NEXT_PUBLIC_BACKEND_URL ?? "";

export type Level = "L0" | "L1" | "L2" | "L3" | "L4" | "FINISH";

export interface GameState {
  level: Level;
}

export interface LoginCredentials {
  username: string;
  password: string;
}

type Msg =
  | ({
      "@type": "GAME_STATE";
    } & MsgGameState)
  | ({ "@type": "LABYRINTH_STATE" } & MsgLabyrinthState)
  | ({ "@type": "ENTER_ENCRYPTION_KEY" } & MsgEnterEncryptionKey)
  | { "@type": "ENCRYPTION_KEY_REJECTED" };

type MsgGameState = {
  level: Level;
};

type MsgLabyrinthState = {
  player1: MsgLabyrinthPlayer;
  player2: MsgLabyrinthPlayer;
};

export type MsgLabyrinthPlayer = {
  position: { x: number; y: number };
  direction: "UP" | "DOWN" | "LEFT" | "RIGHT";
};

type MsgEnterEncryptionKey = {
  key: string;
};

export class BackendConnection {
  private abort: AbortSignal;
  private ws: WebSocket;
  private autoReconnect: boolean = false;
  private updateListeners: Array<() => void>;
  private level: Level | null = null;
  private labyrinthState: MsgLabyrinthState | null = null;
  private pendingEncryptionKeyResponse: ((success: boolean) => void) | null =
    null;

  constructor(ws: WebSocket, abort: AbortSignal) {
    this.abort = abort;
    this.ws = ws;
    this.updateListeners = [];
    this.attachWsListeners();

    abort.addEventListener("abort", () => {
      this.autoReconnect = false;
      this.ws.close();
    });
  }

  static async connect(
    abort: AbortSignal,
    login: LoginCredentials,
  ): Promise<BackendConnection> {
    abort.throwIfAborted();
    const connection = new BackendConnection(createWebSocket(login), abort);
    await connection.waitForConnection();
    // only enable auto-reconnect after the first connection is established
    connection.autoReconnect = true;
    return connection;
  }

  async enterEncryptionKey(key: string, abort: AbortSignal): Promise<void> {
    if (this.pendingEncryptionKeyResponse) {
      throw new Error("Already waiting for encryption key response");
    }

    abort.throwIfAborted();

    let unsubscribe = null as (() => void) | null;
    const promise = new Promise<void>((resolve, reject) => {
      const onTimeout = () => {
        reject(new Error("timeout"));
      };
      const onAbort = () => {
        reject(new Error(abort.reason));
      };

      this.pendingEncryptionKeyResponse = (success) => {
        if (success) {
          resolve();
        } else {
          reject(new Error("Encryption key rejected"));
        }
      };
      const timeoutId = setTimeout(onTimeout, 10000);
      abort.addEventListener("abort", onAbort);

      unsubscribe = () => {
        this.pendingEncryptionKeyResponse = null;
        clearTimeout(timeoutId);
        abort.removeEventListener("abort", onAbort);
      };

      this.ws.send(safeJsonStringify({ "@type": "ENTER_ENCRYPTION_KEY", key }));
    });

    try {
      await promise;
    } finally {
      if (unsubscribe) {
        unsubscribe();
      }
    }
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

  getLabyrinth(): MsgLabyrinthState {
    return this.labyrinthState ?? defaultLabyrinthState;
  }

  private setLabyrinth(labyrinth: MsgLabyrinthState): void {
    const playerEq = (a: MsgLabyrinthPlayer, b: MsgLabyrinthPlayer) => {
      return (
        a.position.x === b.position.x &&
        a.position.y === b.position.y &&
        a.direction === b.direction
      );
    };
    const stateEq = (a: MsgLabyrinthState, b: MsgLabyrinthState) => {
      return playerEq(a.player1, b.player1) && playerEq(a.player2, b.player2);
    };

    const changed =
      this.labyrinthState === null || !stateEq(this.labyrinthState, labyrinth);
    if (!changed) {
      return;
    }

    this.labyrinthState = labyrinth;
    this.dispatchUpdate();
  }

  async waitForUpdate(): Promise<void> {
    let unsubscribe = null as (() => void) | null;
    const promise = new Promise<void>((resolve) => {
      const onUpdate = () => {
        resolve();
      };
      unsubscribe = this.onUpdate(onUpdate);
    });

    try {
      return await promise;
    } finally {
      if (unsubscribe) {
        unsubscribe();
      }
    }
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

  private async waitForConnection(): Promise<void> {
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
        reject(this.abort.reason);
      };
      const onTimeout = () => {
        reject(new Error("Connection timeout"));
      };

      const unsubscribeUpdate = this.onUpdate(onUpdate);
      this.ws.addEventListener("close", onClose);
      this.ws.addEventListener("error", onError);
      this.abort.addEventListener("abort", onAbort);
      const timeoutId = setTimeout(onTimeout, 10000);

      unsubscribe = () => {
        unsubscribeUpdate();
        this.ws.removeEventListener("close", onClose);
        this.ws.removeEventListener("error", onError);
        this.abort.removeEventListener("abort", onAbort);
        clearTimeout(timeoutId);
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

  private reconnect(): void {
    if (!this.autoReconnect) {
      return;
    }

    console.info("reconnecting");
    const oldUrl = this.ws.url;
    this.ws = new WebSocket(oldUrl);
  }

  private attachWsListeners(): void {
    this.ws.binaryType = "arraybuffer";
    this.ws.addEventListener("open", (event) => {
      console.info("open", event);
    });
    this.ws.addEventListener("close", (event) => {
      console.info("close", event);
      this.reconnect();
    });
    this.ws.addEventListener("message", (event) => {
      let data = event.data as string | ArrayBuffer;
      let msg;
      try {
        msg = safeJsonParse(data) as Msg;
      } catch (err) {
        console.error("error parsing message", { event, err });
        return;
      }

      console.info("message", msg);
      switch (msg["@type"]) {
        case "GAME_STATE":
          this.setLevel(msg.level);
          if (this.pendingEncryptionKeyResponse && msg.level === "L4") {
            this.pendingEncryptionKeyResponse(true);
          }
          break;
        case "LABYRINTH_STATE":
          this.setLabyrinth(msg);
          break;
        case "ENCRYPTION_KEY_REJECTED":
          if (this.pendingEncryptionKeyResponse) {
            this.pendingEncryptionKeyResponse(false);
          }
          break;
      }
    });
    this.ws.addEventListener("error", (event) => {
      console.error("error", event);
    });
  }
}

function createWebSocket(login: LoginCredentials): WebSocket {
  const url = new URL(backendUrl, window.location.href);
  url.searchParams.set("username", login.username);
  url.searchParams.set("password", login.password);
  return new WebSocket(url);
}

// should never be seen but we don't want to crash if it's missing
const defaultLabyrinthState: MsgLabyrinthState = {
  player1: { position: { x: 0, y: 0 }, direction: "UP" },
  player2: { position: { x: 0, y: 0 }, direction: "UP" },
};
