import { describe, expect, test } from 'vitest';
import { ValhallaBikeCosting, ValhallaBikeEdge } from './ValhallaBikeFactor';

function edge(overrides: Partial<ValhallaBikeEdge>): ValhallaBikeEdge {
  return {
    use: 'road',
    road_class: 'residential',
    cycle_lane: 'none',
    shoulder: false,
    lane_count: 1,
    truck_route: false,
    bicycle_network: 0,
    weighted_grade: 0,
    surface: 'paved_smooth',
    speed: 18,
    speeds_non_faded: { no_flow: 40 },
    length: 0.05,
    ...overrides,
  };
}

/// Each case is a Seattle edge, with the cost Valhalla 3.8.3 charged for it with no turn onto it.
describe('ValhallaBikeCosting', () => {
  const defaults = new ValhallaBikeCosting();

  test.each([
    [
      'secondary with a dedicated lane on a bike network',
      edge({
        road_class: 'secondary',
        cycle_lane: 'dedicated',
        bicycle_network: 1,
        length: 0.055,
      }),
      17.57,
    ],
    [
      'cycleway on a bike network',
      edge({
        use: 'cycleway',
        road_class: 'service_other',
        cycle_lane: 'dedicated',
        bicycle_network: 1,
        speeds_non_faded: { no_flow: 20 },
        length: 0.044,
      }),
      11.85,
    ],
    [
      'tertiary with sharrows',
      edge({ road_class: 'tertiary', cycle_lane: 'shared', length: 0.036 }),
      14.53,
    ],
  ])('%s', (_, e, valhallaCost) => {
    const model = defaults.factor(e)!;
    expect(model.factor * model.seconds).toBeCloseTo(valhallaCost, 1);
  });

  test('terms sum to the factor', () => {
    const model = defaults.factor(edge({ road_class: 'primary' }))!;
    const sum = model.terms.reduce((total, t) => total + t.value, 0);
    expect(sum).toBeCloseTo(model.factor);
  });

  test('steps are costed separately', () => {
    expect(defaults.factor(edge({ use: 'steps' }))).toBeNull();
  });

  test('road_class_factor scales only the class term', () => {
    const primary = edge({ road_class: 'primary' });
    const term = (costing: ValhallaBikeCosting, label: string) =>
      costing.factor(primary)!.terms.find((t) => t.label === label)!.value;
    const scaled = new ValhallaBikeCosting({ road_class_factor: 5 });
    expect(term(scaled, 'class')).toBeCloseTo(5 * term(defaults, 'class'));
    expect(term(scaled, 'road')).toBeCloseTo(term(defaults, 'road'));
  });

  test('road_class_factors replaces the class weights', () => {
    const costing = new ValhallaBikeCosting({
      road_class_factors: [0, 0, 1, 0, 0, 0, 0, 0],
    });
    const term = (roadClass: string) =>
      costing
        .factor(edge({ road_class: roadClass }))!
        .terms.find((t) => t.label === 'class')!.value;
    expect(term('primary')).toBeCloseTo(1.5);
    expect(term('trunk')).toBe(0);
  });

  test('bicycle_network_factor spares the class term', () => {
    const costing = new ValhallaBikeCosting({ bicycle_network_factor: 0.25 });
    const term = (bicycle_network: number, label: string) =>
      costing
        .factor(edge({ road_class: 'primary', bicycle_network }))!
        .terms.find((t) => t.label === label)!.value;
    expect(term(1, 'class')).toBeCloseTo(term(0, 'class'));
    expect(term(1, 'road')).toBeCloseTo(0.25 * term(0, 'road'));
  });

  test('road_class_factor leaves service roads alone', () => {
    const scaled = new ValhallaBikeCosting({ road_class_factor: 5 });
    const service = edge({ road_class: 'service_other' });
    expect(scaled.factor(service)!.factor).toBeCloseTo(
      defaults.factor(service)!.factor,
    );
  });
});
