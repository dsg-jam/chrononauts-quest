import { Command } from ".";

export default {
  name: "decrypt",
  description: "Set encryption key for device communication",
  async execute({ terminal, backend, args }): Promise<void> {
    const key = args[0];
    if (!key) {
      await terminal.type("Usage: decrypt <key>");
      return;
    }

    await terminal.type("Testing decryption key...");

    try {
      await backend.enterEncryptionKey(key, terminal.abort);
    } catch (err) {
      await terminal.type("Decryption key invalid. Message payload corrupted.");
      return;
    }

    await terminal.type(
      "Message payload decrypted successfully. Encryption key updated.",
    );
  },
} as Command;
