import styles from "./page.module.css";
import { TerminalRenderer } from "@/components/terminal";

export default function Home() {
  return (
    <main className={styles.main}>
      <TerminalRenderer />
    </main>
  );
}
