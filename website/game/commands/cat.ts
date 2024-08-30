import { Command } from ".";

export default {
  name: "cat",
  description: "Exhibit the contents of a particular file.",
  async execute({ terminal, vfs, args }): Promise<void> {
    const path = args[0];
    if (!path) {
      await terminal.type("Usage: cat <path>");
      return;
    }
    const content = await vfs.read(path);
    if (content) {
      await terminal.type(content);
    } else {
      await terminal.type(`No such file: ${path}`);
    }
  },
} as Command;
