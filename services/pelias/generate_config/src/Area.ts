import { default as defaultConfig } from "./defaultConfig.json";
import { PeliasConfig } from "./index";

type ImportsConfig = {
  whosonfirst: WhosOnFirstConfig;
  openaddresses?: OpenAddressesConfig;
};

type WhosOnFirstConfig = {
  datapath: string;
  importPostalcodes: boolean;
  // omit countryCode to import ALL
  countryCode?: string | string[];
};

type OpenAddressesConfig = {
  datapath: string;
  // omit files to import ALL
  files?: string[];
};

export default class Area {
  name: string;
  countryCodes: string[];
  constructor(name: string, countryCodes: string[]) {
    this.name = name;

    countryCodes.forEach((countryCode) =>
      console.assert(countryCode.length > 0),
    );
    this.countryCodes = countryCodes;
  }

  static fromRecord(fields: { area: string; countryCodes: string }): Area {
    return new Area(
      fields.area,
      fields.countryCodes
        .split(",")
        .filter((countryCode) => countryCode.length > 0),
    );
  }

  get isPlanetBuild(): boolean {
    return this.countryCodes[0] == "ALL";
  }

  importsConfig(): ImportsConfig {
    const whosonfirst: WhosOnFirstConfig = {
      datapath: "/data/whosonfirst",
      importPostalcodes: true,
    };

    if (this.countryCodes.length > 0 && !this.isPlanetBuild) {
      // Note: countryCode vs countryCodes
      whosonfirst["countryCode"] = this.countryCodes;
    }

    const importsConfig: ImportsConfig = { whosonfirst };

    if (this.isPlanetBuild) {
      const openaddresses = {
        datapath: "/data/openaddresses",
      };
      importsConfig["openaddresses"] = openaddresses;
    } else {
      const openaddresses = {
        datapath: "/data/openaddresses",
        files: ["bbox_addresses.csv"],
      };
      importsConfig["openaddresses"] = openaddresses;
    }

    return importsConfig;
  }

  peliasConfig(): PeliasConfig {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const config: any = Object.assign({}, defaultConfig);
    const importsConfig = this.importsConfig();
    for (const key in importsConfig) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      config["imports"][key] = (importsConfig as any)[key];
    }
    return config;
  }
}
