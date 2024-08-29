import { allCommands, Command } from ".";

export default {
  name: "ls",
  description: "List all available files",
  async execute({ terminal, vfs }): Promise<void> {
    const paths = await vfs.list();
    for (const path of paths) {
      await terminal.type(`${path}`);
    }
  },
} as Command;
