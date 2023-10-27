import { parse } from "csv-parse/sync";
import Area from "./Area";

// TODO: add types
export type PeliasConfig = any; // eslint-disable-line @typescript-eslint/no-explicit-any

export function generate(
  input: string,
  areaName: string,
  countries: string[],
): PeliasConfig {
  const area = (() => {
    for (const record of parse(input, { columns: true })) {
      const area = Area.fromRecord(record);
      if (area.name != areaName) {
        continue;
      }

      if (countries.length > 0) {
        area.countryCodes = countries;
      }
      return area;
    }
    return new Area(areaName, countries, []);
  })();

  return area.peliasConfig();
}
