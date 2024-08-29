import { allCommands, Command } from ".";

export default {
  name: "help",
  description: "Display this help message",
  async execute({ terminal }): Promise<void> {
    await terminal.type("Available commands:\n");
    for (const command of allCommands) {
      await terminal.type(`  ${command.name} - ${command.description}\n`);
    }
  },
} as Command;
