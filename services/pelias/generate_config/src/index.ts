import { parse } from "csv-parse/sync";
import Area from "./Area";

// TODO: add types
export type PeliasConfig = any; // eslint-disable-line @typescript-eslint/no-explicit-any

export function generate(
  areasDBInput: string,
  areaName: string,
  countries: string[],
): PeliasConfig {
  const area = (() => {
    for (const record of parse(areasDBInput, { columns: true })) {
      const area = Area.fromRecord(record);
      if (area.name != areaName) {
        continue;
      }

      if (countries.length > 0) {
        area.countryCodes = countries;
      }
      return area;
    }

    // No matching area found, return a default one with just the name and countries
    return new Area(areaName, countries, []);
  })();

  return area.peliasConfig();
}
