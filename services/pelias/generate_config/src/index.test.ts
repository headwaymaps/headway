import { readFileSync } from "fs";
import { generate } from "./index";
import * as path from "path";

test("guesses country when missing", () => {
  const inputPath = path.join(path.resolve(__dirname), "../areas.csv");
  const input = readFileSync(inputPath, "utf-8");
  const config = generate(input, "Seattle", []);
  expect(config["imports"]["whosonfirst"]).toEqual({
    countryCode: ["US"],
    datapath: "/data/whosonfirst",
    importPostalcodes: true,
  });
});

test("use country when specified", () => {
  const inputPath = path.join(path.resolve(__dirname), "../areas.csv");
  const input = readFileSync(inputPath, "utf-8");
  const config = generate(input, "Seattle", ["CA"]);
  expect(config["imports"]["whosonfirst"]).toEqual({
    countryCode: ["CA"],
    datapath: "/data/whosonfirst",
    importPostalcodes: true,
  });
});

test("unknown area", () => {
  const inputPath = path.join(path.resolve(__dirname), "../areas.csv");
  const input = readFileSync(inputPath, "utf-8");
  const config = generate(input, "planet-v1.26", ["ALL"]);
  expect(config["imports"]["whosonfirst"]).toEqual({
    datapath: "/data/whosonfirst",
    importPostalcodes: true,
  });
  expect(config["imports"]["openaddresses"]).toEqual({
    datapath: "/data/openaddresses",
  });
});
