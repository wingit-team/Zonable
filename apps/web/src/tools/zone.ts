import { setZone } from '../simulation/zoning';
import type { CityState, ZoneType } from '../types';

export const paintZone = (city: CityState, tileId: string, zone: ZoneType): CityState => setZone(city, tileId, zone);
