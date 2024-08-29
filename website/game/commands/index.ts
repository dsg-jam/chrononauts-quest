import { BackendConnection } from "../backend";
import { Vfs } from "../vfs";
import cat from "./cat";
import conninfo from "./conninfo";
import decrypt from "./decrypt";
import help from "./help";
import locate from "./locate";
import ls from "./ls";
import { Terminal } from "@/components/terminal";

export type Command = {
  name: string;
  description: string;
  execute: (_: {
    terminal: Terminal;
    backend: BackendConnection;
    vfs: Vfs;
    args: string[];
  }) => Promise<void>;
};

export const allCommands: Command[] = [
  cat,
  conninfo,
  decrypt,
  help,
  locate,
  ls,
];
