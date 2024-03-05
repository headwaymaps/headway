// The polline decode method was originally based on the following (though it's changed a bit):

// Copyright (c) 2018 Valhalla contributors
// Copyright (c) 2015-2017 Mapillary AB, Mapzen

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

export function decodePolyline(
  str: string,
  precision: number,
): [number, number][] {
  let index = 0,
    lat = 0,
    lng = 0,
    shift = 0,
    result = 0,
    byte = null,
    latitudeChange,
    longitudeChange;
  const coordinates: [number, number][] = [],
    factor = Math.pow(10, precision);

  // Coordinates have variable length when encoded, so just keep
  // track of whether we've hit the end of the string. In each
  // loop iteration, a single coordinate is decoded.
  while (index < str.length) {
    // Reset shift, result, and byte
    byte = null;
    shift = 0;
    result = 0;

    do {
      byte = str.charCodeAt(index++) - 63;
      result |= (byte & 0x1f) << shift;
      shift += 5;
    } while (byte >= 0x20);

    latitudeChange = result & 1 ? ~(result >> 1) : result >> 1;

    shift = result = 0;

    do {
      byte = str.charCodeAt(index++) - 63;
      result |= (byte & 0x1f) << shift;
      shift += 5;
    } while (byte >= 0x20);

    longitudeChange = result & 1 ? ~(result >> 1) : result >> 1;

    lat += latitudeChange;
    lng += longitudeChange;

    const coord: [number, number] = [lng / factor, lat / factor];
    coordinates.push(coord);
  }

  return coordinates;
}
