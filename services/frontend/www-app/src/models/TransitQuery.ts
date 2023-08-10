import { LocationQuery } from 'vue-router';

export type TransitQueryParams = {
  searchTime?: string;
  searchDate?: string;
  arriveBy?: boolean;
  transitWithBicycle?: boolean;
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
    return query;
  }
}
