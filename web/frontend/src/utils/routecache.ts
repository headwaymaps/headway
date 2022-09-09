import { POI } from './models';
import { ProcessedRouteSummary, Route, summarizeRoute } from './routes';

export type CacheableMode = 'walk' | 'bicycle' | 'car';

function modeToCostingModel(mode: CacheableMode): string {
  switch (mode) {
    case 'walk':
      return 'pedestrian';
    case 'bicycle':
      return 'bicycle';
    case 'car':
      return 'auto';
  }
}

export async function getRoutes(
  from: POI,
  to: POI,
  mode: CacheableMode
): Promise<[Route, ProcessedRouteSummary][]> {
  if (!from.position || !to.position) {
    console.error("Can't request without fully specified endpoints");
    return [];
  }
  const requestObject = {
    locations: [
      {
        lat: from.position.lat,
        lon: from.position.long,
      },
      {
        lat: to.position.lat,
        lon: to.position.long,
      },
    ],
    costing: modeToCostingModel(mode),
    alternates: 3,
  };
  const response = await fetch(
    `/valhalla/route?json=${JSON.stringify(requestObject)}`
  );
  if (response.status !== 200) {
    console.error('Valhalla response gave error: ' + response.status);
    return [];
  }
  const responseJson = await response.json();
  const routes: [Route, ProcessedRouteSummary][] = [];
  const route = responseJson.trip as Route;
  if (route) {
    routes.push([route, summarizeRoute(route)]);
  }
  for (const altIdx in responseJson.alternates) {
    const route = responseJson.alternates[altIdx].trip as Route;
    if (route) {
      routes.push([route, summarizeRoute(route)]);
    }
  }
  return routes;
}
