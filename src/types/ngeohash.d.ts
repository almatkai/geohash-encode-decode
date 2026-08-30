declare module "ngeohash" {
  export function encode(lat: number, lon: number, precision?: number): string;
  export function decode(hash: string): { latitude: number; longitude: number; error: { latitude: number; longitude: number } };
  export function neighbors(hash: string): string[];
  export function bbox(hash: string): { sw: { latitude: number; longitude: number }; ne: { latitude: number; longitude: number } };
}
