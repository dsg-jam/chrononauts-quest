import styles from "./terminal.module.css";
import { ReactNode } from "react";

export function TerminalRenderer({ children }: { children: ReactNode }) {
  return <div className={styles.container}>{children}</div>;
}
