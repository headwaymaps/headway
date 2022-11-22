import { ValhallaRoute } from 'src/services/ValhallaClient';

export default interface Route {
  durationSeconds: number;
  durationFormatted: string;
  viaRoadsFormatted: string;
  lengthFormatted: string;
  valhallaRoute: ValhallaRoute;
}
