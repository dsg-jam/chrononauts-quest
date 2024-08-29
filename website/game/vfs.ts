// TODO: the file system should be backed by the backend so as not to leak anything

export class Vfs {
  constructor() {}

  async list(): Promise<string[]> {
    return Object.keys(files);
  }

  async read(path: string): Promise<string | null> {
    return files[path] ?? null;
  }
}

const files = {
  "notes.txt": "This is a note and it should tell you about how things work",
} as Record<string, string>;
