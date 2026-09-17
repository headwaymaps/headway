import { Bbox } from 'src/services/TransitZonerClient';
import { Result, Ok, Err } from 'src/utils/Result';

/// The schema version `gtfout` writes and refuses to read past.
export const ZONE_VERSION = 1;

/// A zone.json, reduced to what the editor restores from it.
export interface TransitZone {
  readonly bbox: Bbox;
  readonly feedIds: readonly string[];
}

const BOUNDS_FIELDS = ['min_lon', 'min_lat', 'max_lon', 'max_lat'] as const;

/// Reads a saved zone.json back into an editable area and selection.
///
/// The error's message says what is wrong with the file and is meant to be
/// shown: a dropped file that isn't a zone should say why rather than silently
/// leaving the editor unchanged.
export function parseTransitZone(text: string): Result<TransitZone, Error> {
  let document: unknown;
  try {
    document = JSON.parse(text);
  } catch {
    return Err(new Error("That file isn't valid JSON."));
  }
  if (typeof document !== 'object' || document === null) {
    return Err(new Error("That file isn't a zone document."));
  }
  const zone = document as Record<string, unknown>;

  if (zone.version !== ZONE_VERSION) {
    return Err(
      new Error(
        `Zone file is version ${String(zone.version)}, but this tool only understands version ${ZONE_VERSION}.`,
      ),
    );
  }

  const bbox = bboxOf(zone.bounds);
  if (!bbox.ok) {
    return bbox;
  }
  const feedIds = feedIdsOf(zone.feeds);
  if (!feedIds.ok) {
    return feedIds;
  }
  return Ok({ bbox: bbox.value, feedIds: feedIds.value });
}

function bboxOf(bounds: unknown): Result<Bbox, Error> {
  if (typeof bounds !== 'object' || bounds === null) {
    return Err(new Error('Zone file has no bounds.'));
  }
  const corners = bounds as Record<string, unknown>;

  const values: number[] = [];
  for (const field of BOUNDS_FIELDS) {
    const value = corners[field];
    if (typeof value !== 'number' || !Number.isFinite(value)) {
      return Err(new Error(`Zone file has no ${field} in its bounds.`));
    }
    values.push(value);
  }

  const [west, south, east, north] = values as Bbox;
  if (west >= east || south >= north) {
    return Err(new Error('Zone file has empty bounds.'));
  }
  return Ok([west, south, east, north]);
}

function feedIdsOf(feeds: unknown): Result<string[], Error> {
  if (!Array.isArray(feeds)) {
    return Err(new Error('Zone file has no feeds.'));
  }
  const ids: string[] = [];
  for (const [index, feed] of feeds.entries()) {
    const id = (feed as Record<string, unknown> | null)?.feed_onestop_id;
    if (typeof id !== 'string' || !id) {
      return Err(new Error(`Feed ${index + 1} in the zone file has no id.`));
    }
    ids.push(id);
  }
  return Ok(ids);
}
