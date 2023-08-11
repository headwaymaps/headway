import { generate, parseArgs } from "../index";
import { readFileSync } from "fs";

const config = generate(readFileSync(0, "utf-8"), parseArgs());
console.log(JSON.stringify(config, undefined, 2));
