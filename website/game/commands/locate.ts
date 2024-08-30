import { Command } from ".";
import styles from "../../components/labyrinth.module.css";
import { MsgLabyrinthPlayer } from "../backend";
import { showGameComplete, skipBootAnimation } from "../game";
import { sleep } from "@/utils/sleep";

export default {
  name: "locate",
  description: "Tool to locate the devices",
  async execute({ terminal, backend }): Promise<void> {
    if (backend.getLevel() !== "L4" && backend.getLevel() !== "FINISH") {
      await terminal.type([
        "Failed to synchronize position",
        "Connection not established.",
      ]);
      return;
    }

    if (!skipBootAnimation) {
      await terminal.type("Locating devices...", { lineEndDelay: 2000 });
    }

    await terminal.clear();

    const rootEl = terminal.addSubElement();
    rootEl.classList.add(styles.labyrinth);
    rootEl.style.setProperty("--width", labyrinthWidth.toString());
    rootEl.style.setProperty("--height", labyrinthHeight.toString());

    for (const { x, y } of wallPositions) {
      const wallEl = document.createElement("div");
      wallEl.classList.add(styles.wall);
      wallEl.classList.add(styles.positioned);

      wallEl.style.setProperty("--x", x.toString());
      wallEl.style.setProperty("--y", y.toString());
      rootEl.appendChild(wallEl);
      await sleep(5, terminal.abort);
    }

    const player1El = buildPlayerEl("player1");
    rootEl.appendChild(player1El);
    await sleep(100, terminal.abort);
    const player2El = buildPlayerEl("player2");
    rootEl.appendChild(player2El);

    while (true) {
      const state = backend.getLabyrinth();
      updatePlayerEl(player1El, state.player1);
      updatePlayerEl(player2El, state.player2);

      if (backend.getLevel() !== "L4") {
        break;
      }

      await backend.waitForUpdate();
    }

    if (backend.getLevel() !== "FINISH") {
      // somehow we jumped out of the labyrinth level without finishing the game...
      await terminal.type("Connection lost...");
      return;
    }

    await terminal.type("Devices in temporal alignment!", {
      lineEndDelay: 2000,
    });
    await showGameComplete(terminal);
  },
} as Command;

function buildPlayerEl(player: "player1" | "player2"): HTMLElement {
  const playerEl = document.createElement("div");
  playerEl.classList.add(styles.player);
  playerEl.classList.add(styles.positioned);
  switch (player) {
    case "player1":
      playerEl.classList.add(styles.p1);
      break;
    case "player2":
      playerEl.classList.add(styles.p2);
      break;
  }
  return playerEl;
}

function updatePlayerEl(
  playerEl: HTMLElement,
  player: MsgLabyrinthPlayer,
): void {
  playerEl.style.setProperty("--x", player.position.x.toString());
  playerEl.style.setProperty("--y", player.position.y.toString());

  playerEl.classList.remove(styles.up);
  playerEl.classList.remove(styles.down);
  playerEl.classList.remove(styles.left);
  playerEl.classList.remove(styles.right);

  switch (player.direction) {
    case "UP":
      playerEl.classList.add(styles.up);
      break;
    case "DOWN":
      playerEl.classList.add(styles.down);
      break;
    case "LEFT":
      playerEl.classList.add(styles.left);
      break;
    case "RIGHT":
      playerEl.classList.add(styles.right);
      break;
  }
}

// NOTE: copied from backend. DO NOT MODIFY HERE!
// TODO: this should be sent by the backend as part of the labyrinth state, but I'm too lazy to implement that right now.
const rawLabyrinth = `
########################################
####1  ##########################    ###
###### ##        ##   ########### ## ###
###### ## ###### ## # ####        ## ###
##     ## ###### ## # #### ### ##### ###
## ### ##    ### ## ###    ### ##### ###
## ### ##### ###        ###### ##### ###
## ###       ### ############# ##### ###
################       #######2#########
########################################
`;

const wallPositions = rawLabyrinth
  .trim()
  .split("\n")
  .map((line, y) =>
    line
      .split("")
      .map((cell, x) => ({ cell, x, y }))
      .filter(({ cell }) => cell === "#"),
  )
  .flat();

const labyrinthWidth = wallPositions.reduce(
  (max, { x }) => Math.max(max, x + 1),
  0,
);
const labyrinthHeight = wallPositions.reduce(
  (max, { y }) => Math.max(max, y + 1),
  0,
);
