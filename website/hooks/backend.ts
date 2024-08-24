import { useLocalStorage } from "./storage";
import {
  Dispatch,
  SetStateAction,
  useCallback,
  useEffect,
  useState,
} from "react";

const BACKEND_URL = process.env.BACKEND_URL ?? "";

export type Level = "L0" | "L1" | "L2" | "L3" | "L4";
export type ConnectionState = "connecting" | "connected" | "disconnected";

export interface GameState {
  level: Level | null;
}

export interface LoginCredentials {
  username: string;
  password: string;
}

export interface Backend {
  connectionState: ConnectionState;
  game: GameState;
  performLogin: (login: LoginCredentials) => void;
}

export function useBackend(): Backend {
  const [ws, setWs] = useState<WebSocket | null>(null);
  const [game, setGame] = useState<GameState>({ level: null });
  const connectionState = useConnectionState(ws);
  useMsgListener(ws, setGame);
  const performLogin = useLoginEffect(ws, setWs);

  return { connectionState, game, performLogin: performLogin };
}

function useConnectionState(ws: WebSocket | null): ConnectionState {
  const [state, setState] = useState<ConnectionState>("disconnected");
  useEffect(() => {
    if (!ws) {
      return;
    }

    const snapshot = () => {
      switch (ws.readyState) {
        case WebSocket.CONNECTING:
          setState("connecting");
          break;
        case WebSocket.OPEN:
          setState("connected");
          break;
        case WebSocket.CLOSED:
        case WebSocket.CLOSING:
          setState("disconnected");
          break;
      }
    };
    ws.addEventListener("open", snapshot);
    ws.addEventListener("close", snapshot);
    ws.addEventListener("error", snapshot);
    return () => {
      ws.removeEventListener("open", snapshot);
      ws.removeEventListener("close", snapshot);
      ws.removeEventListener("error", snapshot);
    };
  }, [ws, setState]);

  return state;
}

type Msg = {
  "@type": "GAME_STATE";
} & MsgGameState;

type MsgGameState = {
  level: Level;
};

function useMsgListener(
  ws: WebSocket | null,
  setGame: Dispatch<SetStateAction<GameState>>,
) {
  useEffect(() => {
    if (!ws) {
      return;
    }
    const onMessage = (event: MessageEvent) => {
      const msg: Msg = JSON.parse(event.data);
      switch (msg["@type"]) {
        case "GAME_STATE":
          setGame((game) => ({ ...game, level: msg.level }));
          break;
      }
    };
    ws.addEventListener("message", onMessage);
    return () => ws.removeEventListener("message", onMessage);
  }, [ws, setGame]);
}

function useLoginEffect(
  ws: WebSocket | null,
  setWs: Dispatch<SetStateAction<WebSocket | null>>,
): (login: LoginCredentials) => void {
  const [{ username, password }, setLogin] = useLocalStorage("login", {
    username: null as string | null,
    password: null as string | null,
  });
  useEffect(() => {
    if (!username || !password) {
      return;
    }
    const ws = createWebSocket(username, password);
    setWs(ws);
  }, [ws, username, password]);
  const performLogin = useCallback(
    (login: LoginCredentials) => {
      setLogin(login);
    },
    [setLogin],
  );
  return performLogin;
}

function createWebSocket(username: string, password: string): WebSocket {
  const url = new URL(BACKEND_URL, window.location.href);
  url.searchParams.set("username", username);
  url.searchParams.set("password", password);
  const ws = new WebSocket(url);
  return ws;
}
