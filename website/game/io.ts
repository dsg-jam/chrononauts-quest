export interface Io {
  type(opts: { abort: AbortSignal; text: string }): Promise<void>;
  input(opts: { abort: AbortSignal; hidden?: boolean }): Promise<string>;
  prompt(opts: {
    abort: AbortSignal;
    text: string;
    hidden?: boolean;
  }): Promise<string>;
}
