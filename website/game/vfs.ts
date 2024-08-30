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
  // first note: let the player know that these notes are written by "the scientist" and lampshade the limited functionality of the "operating system"
  "200827.txt": `
I am creating this note to ascertain the capability of my new
operating system to successfully mount the file system in
read-only mode.
Presently, I lack the fortitude to undertake the implementation
of write support.
Therefore, I shall proceed with the mounting of the files from
the mainframe.
`,
  // second note: hint the player to learn about the padlock and encryption key and give them the necessary clue that the encryption key can be decrypted manually (since the one on disk is corrupted)
  "200828.txt": `
The preceding memorandum has successfully traversed the necessary
channels, thus enabling me to securely archive the codes that
keep eluding my memory.
It is exceedingly vexatious to gather the decryption key by hand.

I shall retain the 'decrypt' program in its current position,
for I have invested an inordinate amount of time in its
meticulous configuration.
`,
  // other files: padlock should be used to get the devices out of the box and the encryption key is a false lead. The players get to do it by hand :)
  "encryption_key.txt": `ERROR: file mount corrupted. Unable to read blocks 22300 to 22310.`,
  "padlock.txt": `697`,
  // third note: teach the player what the actual goal of the game is and at least hint at how the scientist died.
  "200830.txt": `
The 'locate' command has at long last achieved operational status;
however, I find myself unable to execute the requisite steps
independently.
It is imperative that both devices maintain temporal alignment,
yet I possess the means to control but a single apparatus.
Consequently, I shall venture forth on the morrow in search of a
willing collaborator.
Given that the townsfolk regard me as somewhat unhinged, I harbor
concerns for my personal safety.

Nevertheless, I remain optimistic. I am imbued with a favorable
premonition that this endeavor shall culminate in the successful
completion of my work.
`,
} as Record<string, string>;
