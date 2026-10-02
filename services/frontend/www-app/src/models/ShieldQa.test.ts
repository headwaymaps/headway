import { expect, test } from 'vitest';
import { readFileSync } from 'node:fs';
import type { StyleSpecification } from 'maplibre-gl';
import ShieldQa from './ShieldQa';

const style: StyleSpecification = JSON.parse(
  readFileSync(
    new URL(
      '../../../../tileserver/assets/styles/basic-v3.json',
      import.meta.url,
    ),
    'utf8',
  ),
);

test('finds every network the road_shield layer has a blank for', () => {
  const qa = ShieldQa.fromStyle(style);
  const networks = qa.networks();
  expect(networks).toEqual(expect.arrayContaining(['US:I', 'US:WA', 'MX:MX']));
  expect(new Set(networks).size).toBe(networks.length);
  expect(qa.adjustments(networks[0]!)).toEqual(expect.any(String));
});
