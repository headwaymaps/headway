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
  openaddressesFiles: string[];
  constructor(
    name: string,
    countryCodes: string[],
    openaddressesFiles: string[],
  ) {
    this.name = name;

    countryCodes.forEach((countryCode) =>
      console.assert(countryCode.length > 0),
    );
    this.countryCodes = countryCodes;

    openaddressesFiles.forEach((file) => console.assert(file.length > 0));
    this.openaddressesFiles = openaddressesFiles;
  }

  static fromRecord(fields: {
    name: string;
    countryCodes: string;
    openaddressesFiles: string;
  }): Area {
    return new Area(
      fields.name,
      fields.countryCodes
        .split(",")
        .filter((countryCode) => countryCode.length > 0),
      fields.openaddressesFiles.split(",").filter((file) => file.length > 0),
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
    } else if (this.openaddressesFiles.length > 0) {
      const openaddresses = {
        datapath: "/data/openaddresses",
        files: this.openaddressesFiles,
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
