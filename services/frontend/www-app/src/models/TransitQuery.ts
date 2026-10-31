import { LngLat } from 'maplibre-gl';
import { LocationQuery } from 'vue-router';

export type TransitQueryParams = {
  searchTime?: string;
  searchDate?: string;
  arriveBy?: boolean;
  transitWithBicycle?: boolean;
  /// Places the traveler dragged the route through, in order.
  vias?: LngLat[];
};

export default class TransitQuery {
  params: TransitQueryParams;
  constructor(params: TransitQueryParams) {
    this.params = params;
  }

  static parseFromQuery(query: LocationQuery): TransitQuery {
    const params: TransitQueryParams = {};

    if (typeof query.searchTime == 'string') {
      params.searchTime = query.searchTime;
    }
    if (typeof query.searchDate == 'string') {
      params.searchDate = query.searchDate;
    }
    if (typeof query.arriveBy == 'string') {
      params.arriveBy = query.arriveBy === 'true';
    }
    if (typeof query.transitWithBicycle == 'string') {
      params.transitWithBicycle = query.transitWithBicycle === 'true';
    }
    if (typeof query.via == 'string') {
      params.vias = query.via.split(';').flatMap((latLon) => {
        const [lat, lng] = latLon.split(',').map(Number);
        return Number.isFinite(lat) && Number.isFinite(lng)
          ? [new LngLat(lng!, lat!)]
          : [];
      });
    }

    return new TransitQuery(params);
  }
  searchQuery(): Record<string, string> {
    const query: Record<string, string> = {};
    if (this.params.searchDate) {
      query['searchDate'] = this.params.searchDate;
    }
    if (this.params.searchTime) {
      query['searchTime'] = this.params.searchTime;
    }
    if (this.params.arriveBy) {
      query['arriveBy'] = this.params.arriveBy.toString();
    }
    if (this.params.transitWithBicycle) {
      query['transitWithBicycle'] = this.params.transitWithBicycle.toString();
    }
    if (this.params.vias?.length) {
      query['via'] = this.params.vias
        .map(({ lat, lng }) => `${lat.toFixed(6)},${lng.toFixed(6)}`)
        .join(';');
    }
    return query;
  }
}
