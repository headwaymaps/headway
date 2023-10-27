import { generate } from "../index";
import { readFileSync } from "fs";

export function parseArgs(args: string[]): [string, string, string[]] {
  const USAGE = `ARGS: \n <inputPath> <area> [<countries ...>]`;

  const inputPath = args.shift();
  if (!inputPath) {
    console.error(USAGE);
    throw Error("Missing `inputPath` arg");
  }

  const area = args.shift();
  if (!area) {
    console.error(USAGE);
    throw Error("Missing `area` arg");
  }

  const countries = args;
  return [inputPath, area, countries];
}

// Get just the args from `node $cmd [args...]`
const args = process.argv.slice(2);
const [inputPath, area, countries] = parseArgs(args);
const input = readFileSync(inputPath, "utf-8");
const config = generate(input, area, countries);
console.log(JSON.stringify(config, undefined, 2));
