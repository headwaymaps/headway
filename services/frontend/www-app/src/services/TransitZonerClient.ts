/// The transit-zoner API, which serves the measured GTFS feed index.
///
/// It's mounted behind the same nginx as this app, so these are same-origin
/// paths rather than a configured host.

/// [min_lon, min_lat, max_lon, max_lat], the order every endpoint speaks.
export type Bbox = [number, number, number, number];

/// A realtime feed riding along with a static one.
export interface RealtimeSummary {
  feed_id: string;
  /** Which streams it publishes, e.g. "trip updates", "vehicle positions". */
  kinds: string[];
  authorization_type: string;
  info_url: string | null;
}

/// One feed as `/api/feeds-by-bbox` returns it.
export interface FeedSummary {
  feed_id: string;
  provider: string;
  url: string;
  authorization_type: string;
  info_url: string | null;
  realtime: RealtimeSummary[];
  bbox: Bbox;
  area_m2: number;
  /** How well the extent matches the drawn area, 0 to 1. */
  relevance: number | null;
}

const ROOT = '/transit-zoner/api';

/// What to show a person when a request fails.
///
/// transit-zoner's own errors are plain text and say something actionable
/// ("bbox must be min_lon,min_lat,max_lon,max_lat"). Anything else came from a
/// proxy in between - an nginx or Cloudflare error page when the service is
/// down - and pasting its HTML into the UI helps nobody.
async function errorFrom(response: Response): Promise<Error> {
  const contentType = response.headers.get('content-type') ?? '';
  if (contentType.startsWith('text/plain')) {
    const body = (await response.text()).trim();
    if (body) {
      return new Error(body);
    }
  }
  const status = response.statusText
    ? `${response.status} ${response.statusText}`
    : `${response.status}`;
  return new Error(`Couldn't reach transit-zoner (${status})`);
}

async function getJson<T>(url: string): Promise<T> {
  const response = await fetch(url);
  if (!response.ok) {
    throw await errorFrom(response);
  }
  return (await response.json()) as T;
}

export default class TransitZonerClient {
  /// The feeds whose measured extents touch the drawn area, best match first.
  static async feedsByBbox(bbox: Bbox): Promise<FeedSummary[]> {
    return getJson(`${ROOT}/feeds-by-bbox?bbox=${bbox.join(',')}`);
  }

  /// The zone document for an area and a chosen set of feeds. Returned as text
  /// rather than parsed: it's what gets saved, and the server's formatting is
  /// the formatting that lands in the file.
  static async zoneDocument(bbox: Bbox, feedIds: string[]): Promise<string> {
    const response = await fetch(`${ROOT}/zone`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ bbox: bbox.join(','), feed_ids: feedIds }),
    });
    if (!response.ok) {
      throw await errorFrom(response);
    }
    return await response.text();
  }
}
