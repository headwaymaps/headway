export default class Config {
  transitRoutingEnabled = false;
  maxBounds: [number, number, number, number] | null = null;
  routerLinks?: Record<string, string>;
  aboutUrl?: string;
  aboutLinkText?: string;
  contactUrl?: string;
  contactLinkText?: string;

  static shared: Config = new Config();

  public static get transitRoutingEnabled(): boolean {
    return Config.shared.transitRoutingEnabled;
  }

  public static get maxBounds(): [number, number, number, number] | null {
    return Config.shared.maxBounds;
  }

  /// Where to read about each OTP router, by name.
  public static get routerLinks(): Record<string, string> | undefined {
    return Config.shared.routerLinks;
  }

  public static get aboutUrl(): string | undefined {
    return Config.shared.aboutUrl;
  }

  public static get aboutLinkText(): string | undefined {
    return Config.shared.aboutLinkText;
  }

  public static get contactUrl(): string | undefined {
    return Config.shared.contactUrl;
  }

  public static get contactLinkText(): string | undefined {
    return Config.shared.contactLinkText;
  }
}
