export default class Config {
  transitRoutingEnabled = false;
  maxBounds: [number, number, number, number] | null = null;

  static shared: Config = new Config();

  public static get transitRoutingEnabled(): boolean {
    return Config.shared.transitRoutingEnabled;
  }

  public static get maxBounds(): [number, number, number, number] | null {
    return Config.shared.maxBounds;
  }
}
