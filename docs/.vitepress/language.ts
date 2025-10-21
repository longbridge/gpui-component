import { readFileSync } from "fs";

const lightTheme = JSON.parse(readFileSync("src/light.theme.json").toString());
const darkTheme = JSON.parse(readFileSync("src/dark.theme.json").toString());

export { darkTheme, lightTheme };
