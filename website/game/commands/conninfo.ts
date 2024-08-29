import { Command } from ".";

export default {
  name: "conninfo",
  description: "Print the connection information",
  async execute({ terminal, backend }): Promise<void> {
    const level = backend.getLevel();
    switch (level) {
      // players haven't connected the board to wifi yet
      case "L0":
      case "L1":
        await terminal.type("No communication device detected.");
        break;
      // players haven't synched the board frequency yet
      case "L2":
        await terminal.type("D1: detected");
        await terminal.type("D2: missing");
        await terminal.type([
          "",
          "Frequency mismatch.",
          "Communication corrupted.",
        ]);
        break;
      // players haven't decoded encryption key yet
      case "L3":
        await terminal.type("D1: detected");
        await terminal.type("D2: detected");
        await terminal.type("SYNC: corrupted");
        await terminal.type([
          "",
          "Sync payload mismatch.",
          "Encryption code might be invalid. Use 'decrypt' command to update it.",
        ]);
        break;
      default:
        await terminal.type("D1: detected");
        await terminal.type("D2: detected");
        await terminal.type("SYNC: complete");
        await terminal.type(["", "Connection established."]);
    }
  },
} as Command;
