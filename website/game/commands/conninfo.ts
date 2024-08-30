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
        await terminal.type([
          "D1: missing",
          "D2: missing",
          "SYNC: N/A",
          "",
          "No communication device detected.",
        ]);
        break;
      // players haven't synched the board frequency yet
      case "L2":
        await terminal.type([
          "D1: detected",
          "D2: missing",
          "SYNC: N/A",
          "",
          "Frequency mismatch.",
          "Communication corrupted.",
        ]);
        break;
      // players haven't decoded encryption key yet
      case "L3":
        await terminal.type([
          "D1: detected",
          "D2: detected",
          "SYNC: corrupted",
          "",
          "Sync payload mismatch.",
          "Encryption code might be invalid. Use 'decrypt' command to update it.",
        ]);
        break;
      default:
        await terminal.type([
          "D1: detected",
          "D2: detected",
          "SYNC: complete",
          "",
          "Connection established.",
        ]);
    }
  },
} as Command;
