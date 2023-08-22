import { parse } from "csv-parse/sync";
import Area from "./Area";

type Args = {
  area: string;
  countries: string[];
};

// TODO: add types
export type PeliasConfig = any; // eslint-disable-line @typescript-eslint/no-explicit-any

export function parseArgs(): Args {
  const args = process.argv.slice(2);

  const area = args[0];
  if (!area) {
    throw Error("Missing area arg");
  }

  const countries = args.slice(1);
  return { area, countries };
}

export function generate(input: string, args: Args): PeliasConfig {
  const area = (() => {
    for (const record of parse(input, { columns: true })) {
      const area = Area.fromRecord(record);
      if (area.name != args.area) {
        continue;
      }

      if (args.countries.length > 0) {
        area.countryCodes = args.countries;
      }
      return area;
    }
    return new Area(args.area, args.countries, []);
  })();

  return area.peliasConfig();
}
