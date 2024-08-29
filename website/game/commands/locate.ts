import { Command } from ".";

export default {
  name: "locate",
  description: "Tool to locate the INSERT NAME HERE device",
  async execute({ terminal, backend }): Promise<void> {
    if (backend.getLevel() !== "L4") {
      await terminal.type([
        "Failed to synchronize position",
        "Connection not established.",
      ]);
      return;
    }

    // TODO labyrinth level
  },
} as Command;
