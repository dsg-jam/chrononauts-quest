"use client";

import styles from "./terminal.module.css";
import { createContext, ReactNode, useContext, useMemo } from "react";

const TerminalContext = createContext<TerminalOperations | null>(null);

export interface TerminalOperations {
  input(abortSignal: AbortSignal, hidden: boolean): Promise<string>;
}

export function Terminal({ children }: { children: ReactNode }) {
  const terminal = useMemo(() => buildTerminalOperations(), []);
  return (
    <div className={styles.container}>
      <TerminalContext.Provider value={terminal}>
        {children}
      </TerminalContext.Provider>
    </div>
  );
}

function buildTerminalOperations(): TerminalOperations {
  return {
    async input(abortSignal: AbortSignal, hidden: boolean) {
      return new Promise((resolve, reject) => {
        const input = prompt("Enter a value");
        if (input === null) {
          reject(new Error("User cancelled"));
        } else {
          resolve(input);
        }
      });
    },
  };
}

export function useTerminal(): TerminalOperations {
  const terminalCtx = useContext(TerminalContext);
  if (!terminalCtx) {
    throw new Error("useTerminal must be used within a Terminal");
  }
  return terminalCtx;
}
