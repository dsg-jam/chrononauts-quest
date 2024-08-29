// TODO: the file system should be backed by the backend so as not to leak anything
// this can be implemented by just sending a 'VfsList' and 'VfsRead' request to the backend and then return the response

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
  "notes.txt":
    "This is a note and it should tell you about how things work.\nThe password you're looking for is '1234' blabla.",
} as Record<string, string>;
