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

    await terminal.type("Locating devices...");

    console.info(wallPositions);

    // TODO labyrinth level
  },
} as Command;

// NOTE: copied from backend. DO NOT MODIFY HERE!
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
  .split("\n")
  .map((line, y) =>
    line
      .split("")
      .map((cell, x) => ({ cell, x, y }))
      .filter(({ cell }) => cell === "#"),
  )
  .flat();
