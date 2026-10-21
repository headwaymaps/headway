import { execFileSync } from 'node:child_process';
import { expect, test } from 'vitest';
import { fileURLToPath } from 'node:url';
import type { StyleSpecification } from 'maplibre-gl';
import ShieldQa, { SHIELD_QA_COLUMNS } from './ShieldQa';

const style: StyleSpecification = JSON.parse(
  execFileSync(
    'cargo',
    [
      'run',
      '--locked',
      '--quiet',
      '-p',
      'maplibre-style',
      '--bin',
      'headway-style',
      'v4',
    ],
    {
      cwd: fileURLToPath(new URL('../../../../../', import.meta.url)),
      encoding: 'utf8',
    },
  ),
);

test('finds every network the road_shield layer has a blank for', () => {
  const qa = ShieldQa.fromStyle(style);
  const networks = qa.networks();
  expect(networks).toEqual(expect.arrayContaining(['US:I', 'US:WA', 'MX:MX']));
  expect(new Set(networks).size).toBe(networks.length);
  expect(qa.adjustments(networks[0]!)).toEqual(expect.any(String));
});

test("draws every row on a motorway, with the style's own road layers", () => {
  const qa = ShieldQa.fromStyle(style);
  const rowCount = qa.rows('').length;
  const motorways = qa
    .scenery()
    .transportation.features.filter(
      (feature) => feature.properties?.class === 'motorway',
    );
  expect(motorways).toHaveLength(rowCount);

  const layers = qa.style(qa.rows('')).layers;
  const motorway = layers.find((layer) => layer.id === 'road_motorway');
  expect(motorway).toMatchObject({
    source: 'shield-qa-scenery-transportation',
  });
  expect(motorway).not.toHaveProperty('source-layer');
  expect(layers.at(-1)?.id).toBe('road_shield');
  expect(layers.at(-1)).toMatchObject({
    layout: {
      'text-field': expect.arrayContaining([
        'case',
        ['get', 'shield_qa_no_text'],
        '',
      ]),
    },
  });
});

test('adds a no-text shield column', () => {
  const qa = ShieldQa.fromStyle(style);
  const features = qa.features(['US:WA']).features;
  expect(features).toHaveLength(SHIELD_QA_COLUMNS.length);
  expect(
    features.find((feature) => feature.properties?.shield_qa_no_text),
  ).toMatchObject({ properties: { route_1_ref: '5' } });
});
