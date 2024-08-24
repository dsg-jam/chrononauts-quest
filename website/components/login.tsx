"use client";

import { TerminalOperations, useTerminal } from "./terminal";
import { LoginCredentials, useBackend } from "@/hooks/backend";
import { useEffect } from "react";

export function Login() {
  const terminal = useTerminal();
  const { connectionState, performLogin } = useBackend();
  useEffect(() => {
    const abortController = new AbortController();
    run({ terminal, abortSignal: abortController.signal, performLogin });
    return () => abortController.abort();
  }, []);

  return <>{connectionState}</>;
}

async function run({
  terminal,
  abortSignal,
  performLogin,
}: {
  terminal: TerminalOperations;
  abortSignal: AbortSignal;
  performLogin: (login: LoginCredentials) => void;
}) {
  const username = await terminal.input(abortSignal, false);
  const password = await terminal.input(abortSignal, true);
  performLogin({ username, password });
}
