"use client";

import styles from "./terminal.module.css";
import { run } from "@/game/game";
import { sleep } from "@/utils/sleep";
import { useEffect, useRef } from "react";

export function TerminalRenderer() {
  const terminalEl = useRef<HTMLDivElement>(null);
  const terminal = useRef<Terminal | null>(null);

  useEffect(() => {
    if (!terminalEl.current) {
      return;
    }

    const abortController = new AbortController();
    terminal.current = new Terminal(abortController.signal, terminalEl.current);
    run(terminal.current).catch(() => {});

    return () => {
      abortController.abort();
    };
  }, [terminalEl.current]);

  return (
    <div
      className={[styles.container, styles["theme-green"]].join(" ")}
      onClick={() => terminal.current?.focusInput()}
    >
      <div id={styles.monitor}>
        <div id={styles.screen}>
          <div id={styles.crt}>
            <div className={styles.scanline}></div>
            <div className={styles.terminal} ref={terminalEl}></div>
          </div>
        </div>
      </div>
    </div>
  );
}

export class Terminal {
  abort: AbortSignal;
  private terminalEl: HTMLElement;
  private historyOldestFirst: string[];

  constructor(abort: AbortSignal, terminalEl: HTMLElement) {
    this.abort = abort;
    this.terminalEl = terminalEl;
    this.historyOldestFirst = [];
  }

  addToHistory(text: string) {
    this.historyOldestFirst.push(text);
  }

  historyNewestFirst() {
    return this.historyOldestFirst.slice().reverse();
  }

  clear(): void {
    this.terminalEl.innerHTML = "";
  }

  scrollToBottom(): void {
    this.terminalEl.scrollTop = this.terminalEl.scrollHeight;
  }

  focusInput(): void {
    const input = this.terminalEl.querySelector("[contenteditable='true']");
    if (input instanceof HTMLElement) {
      input.focus();
    }
  }

  async type(
    text: string | string[],
    opts?: {
      startDelay?: number;
      charDelay?: number;
      lineEndDelay?: number;
    },
  ): Promise<void> {
    let { startDelay, charDelay, lineEndDelay } = opts ?? {};

    const lines = Array.isArray(text) ? text : [text];
    let firstLine = false;
    for (const line of lines) {
      await this.typeLine(line, {
        startDelay: firstLine ? startDelay : 0,
        charDelay,
        endDelay: lineEndDelay,
      });
      firstLine = false;
    }
  }

  async typeLine(
    text: string,
    opts?: {
      startDelay?: number;
      charDelay?: number;
      endDelay?: number;
    },
  ): Promise<void> {
    let { startDelay = 1000, charDelay = 30, endDelay = 500 } = opts ?? {};

    const lineEl = document.createElement("div");
    lineEl.classList.add(styles.typer);
    lineEl.classList.add(styles.active);
    this.terminalEl.appendChild(lineEl);

    if (startDelay) {
      await sleep(startDelay, this.abort);
    }

    const chars = text.split("");
    let firstChar = true;
    for (const char of chars) {
      if (firstChar) {
        firstChar = false;
      } else {
        await sleep(charDelay, this.abort);
      }

      switch (char) {
        case "\n":
          lineEl.innerHTML += "<br>";
          break;
        case "\t":
          lineEl.innerHTML += "&nbsp;&nbsp;";
          break;
        case " ":
          lineEl.innerHTML += "&nbsp;";
          break;
        default:
          lineEl.textContent += char;
      }
    }

    if (endDelay) {
      await sleep(endDelay, this.abort);
    }

    lineEl.classList.remove(styles.active);
  }

  async input(hidden?: boolean): Promise<string> {
    const inputEl = document.createElement("span");
    inputEl.id = styles.input;
    inputEl.contentEditable = "true";
    if (hidden) {
      inputEl.classList.add(styles.password);
    }

    this.terminalEl.appendChild(inputEl);
    inputEl.focus();

    try {
      return await inputReader({
        el: inputEl,
        abort: this.abort,
        hidden: !!hidden,
        historyNewestFirst: this.historyNewestFirst(),
      });
    } finally {
      inputEl.contentEditable = "false";
    }
  }

  async prompt(text: string, hidden?: boolean): Promise<string> {
    await this.type(text);
    return await this.input(hidden);
  }
}

async function inputReader({
  el,
  abort,
  hidden,
  historyNewestFirst,
}: {
  el: HTMLElement;
  abort: AbortSignal;
  hidden: boolean;
  historyNewestFirst: string[];
}): Promise<string> {
  abort.throwIfAborted();

  const state = {
    lineBuf: null as string | null,
    historyIndex: -1,
  };

  let unsubscribe = null as (() => void) | null;
  const promise = new Promise<string>((resolve, reject) => {
    const onPrintableKey = (key: string) => {
      // Wrap the character in a span
      let span = document.createElement("span");
      // Add span to the input
      span.classList.add(styles.char);
      span.textContent = key;
      el.appendChild(span);

      // For password field, fill the data-pw attr with asterisks
      // which will be shown using CSS
      if (hidden) {
        let length = el.textContent?.length;
        el.setAttribute("data-pw", Array(length).fill("*").join(""));
      }
      // moveCaretToEnd(this.el);
    };

    const onKeyDown = (ev: KeyboardEvent) => {
      switch (ev.key) {
        case "Enter":
          ev.preventDefault();
          resolve(cleanInput(el.textContent ?? ""));
          break;
        case "ArrowUp":
          if (state.historyIndex === -1) {
            state.lineBuf = el.textContent;
          }
          state.historyIndex = Math.min(
            historyNewestFirst.length - 1,
            state.historyIndex + 1,
          );
          el.textContent = historyNewestFirst[state.historyIndex];
          break;
        case "ArrowDown":
          state.historyIndex = Math.max(-1, state.historyIndex - 1);
          el.textContent =
            historyNewestFirst[state.historyIndex] ?? state.lineBuf;
          break;
        case "Backspace":
          // Prevent inserting a <br> when removing the last character
          if (el.textContent?.length === 1) {
            ev.preventDefault();
            el.innerHTML = "";
          }
          break;
        default:
          if (isPrintable(ev.keyCode) && !ev.ctrlKey) {
            ev.preventDefault();
            onPrintableKey(ev.key);
          }
          break;
      }
    };
    const onAbort = () => {
      reject(abort.reason);
    };

    el.addEventListener("keydown", onKeyDown);
    abort.addEventListener("abort", onAbort);

    unsubscribe = () => {
      el.removeEventListener("keydown", onKeyDown);
      abort.removeEventListener("abort", onAbort);
    };
  });

  try {
    return await promise;
  } finally {
    if (unsubscribe) {
      unsubscribe();
    }
  }
}

function cleanInput(input: string): string {
  return input.toLowerCase().trim();
}

function isPrintable(keycode: number) {
  return (
    (keycode > 47 && keycode < 58) || // number keys
    keycode === 32 || // spacebar & return key(s) (if you want to allow carriage returns)
    (keycode > 64 && keycode < 91) || // letter keys
    (keycode > 95 && keycode < 112) || // numpad keys
    (keycode > 185 && keycode < 193) || // ;=,-./` (in order)
    (keycode > 218 && keycode < 223)
  );
}
