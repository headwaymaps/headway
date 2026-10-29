/// Valhalla's bicycle edge factor (cost per second), rebuilt term by term from a trace_attributes
/// edge. Mirrors `BicycleCost::EdgeCost` at Valhalla 3.8.3 (plus our fork's `road_class_factor`,
/// `road_class_factors` and `bicycle_network_factor`, which spares the road class's stress) for a
/// hybrid bike.

/// In the order `road_class_factors` takes them.
const ROAD_CLASSES = [
  'motorway',
  'trunk',
  'primary',
  'secondary',
  'tertiary',
  'unclassified',
  'residential',
  'service_other',
];
const DEFAULT_ROAD_CLASS_FACTORS = [1.0, 0.4, 0.2, 0.1, 0.05, 0.05, 0.0, 0.5];
/// `road_class_factor` scales motorway through tertiary.
const MAJOR_ROAD_CLASSES = ROAD_CLASSES.slice(0, 5);
const CYCLE_LANES = ['none', 'shared', 'dedicated', 'separated'];
const AVOID_HILLS_STRENGTH = [
  2.0, 1.0, 0.5, 0.2, 0.1, 0.0, 0.05, 0.1, 0.3, 0.8, 2.0, 3.0, 4.5, 6.5, 10.0,
  12.0,
];
const SURFACES = [
  'paved_smooth',
  'paved',
  'paved_rough',
  'compacted',
  'dirt',
  'gravel',
  'path',
  'impassable',
];
/// Hybrid bikes are penalized from dirt onward.
const FIRST_BAD_SURFACE = SURFACES.indexOf('dirt');
const SURFACE_FACTORS = [1.0, 2.5, 4.5, 7.0];
const TRUCK_STRESS = 0.5;

/// The bicycle `costing_options` the model accounts for.
export type ValhallaBikeOptions = {
  use_roads: number;
  use_hills: number;
  avoid_bad_surfaces: number;
  road_class_factor: number;
  road_class_factors: number[];
  bicycle_network_factor: number;
};

export const DEFAULT_BIKE_OPTIONS: ValhallaBikeOptions = {
  use_roads: 0.25,
  use_hills: 0.25,
  avoid_bad_surfaces: 0.25,
  road_class_factor: 1,
  road_class_factors: DEFAULT_ROAD_CLASS_FACTORS,
  bicycle_network_factor: 0.95,
};

export type ValhallaBikeEdge = {
  use: string;
  road_class: string;
  cycle_lane?: string;
  shoulder?: boolean;
  lane_count?: number;
  truck_route?: boolean;
  bicycle_network?: number;
  /// Percent, as trace_attributes reports it.
  weighted_grade: number;
  surface: string;
  /// The bike's speed on the edge, kph.
  speed: number;
  /// `no_flow` is the edge's car speed, kph.
  speeds_non_faded: { no_flow: number };
  /// km
  length: number;
};

export type ValhallaBikeFactor = {
  factor: number;
  /// Additive terms that sum to `factor`.
  terms: { label: string; value: number }[];
  /// How much the edge's bike infrastructure scales the road's stress.
  accommodation: number;
  /// How much being on a bike network scales the stress of all but the road's class.
  network: number;
  /// How much the edge's car speed scales the road's stress.
  speedPenalty: number;
  /// The edge's own travel time, without the turn onto it.
  seconds: number;
};

/// Valhalla's bicycle costing for one set of costing options.
export class ValhallaBikeCosting {
  private readonly options: ValhallaBikeOptions;
  private readonly roadFactor: number;
  /// Indexed by shoulder * 4 + cycle lane.
  private readonly cycleLaneFactor: number[];
  private readonly pathCycleLaneFactor: number[];

  constructor(options: Partial<ValhallaBikeOptions> = {}) {
    this.options = { ...DEFAULT_BIKE_OPTIONS, ...options };
    const u = this.options.use_roads;
    this.roadFactor = u >= 0.5 ? 1.5 - u : 2 - u * 2;
    this.cycleLaneFactor = [
      1.0,
      0.9 + u * 0.05,
      0.4 + u * 0.45,
      0.15 + u * 0.6,
      0.7 + u * 0.2,
      0.9 + u * 0.05,
      0.4 + u * 0.45,
      0.15 + u * 0.6,
    ];
    this.pathCycleLaneFactor = [0.2 + u, 0.2 + u, 0.1 + u * 0.9, u * 0.8];
  }

  private speedPenalty(kph: number): number {
    if (kph === 0) return 0;
    const avoidRoads = (1 - this.options.use_roads) * 0.75 + 0.25;
    const base =
      kph <= 40 ? kph / 40 : kph <= 65 ? kph / 25 - 0.6 : kph / 50 + 0.7;
    return (base - 1) * avoidRoads + 1;
  }

  /// Null for steps and ferries, which Valhalla costs separately.
  factor(edge: ValhallaBikeEdge): ValhallaBikeFactor | null {
    if (edge.use === 'steps' || edge.use === 'ferry') return null;

    const {
      use_roads,
      use_hills,
      avoid_bad_surfaces,
      road_class_factor,
      road_class_factors,
      bicycle_network_factor,
    } = this.options;
    const lane = CYCLE_LANES.indexOf(edge.cycle_lane ?? 'none');
    let accommodation = 1;
    let penalty = 1;
    let stressTerms: { label: string; value: number }[];
    if (['cycleway', 'footway', 'path'].includes(edge.use)) {
      accommodation = this.pathCycleLaneFactor[lane]!;
      stressTerms = [{ label: edge.use, value: 1 }];
    } else if (edge.use === 'living_street') {
      stressTerms = [{ label: 'living street', value: 0.2 + use_roads * 0.8 }];
    } else if (edge.use === 'track') {
      stressTerms = [{ label: 'track', value: 0.5 + use_roads }];
    } else {
      accommodation = this.cycleLaneFactor[(edge.shoulder ? 4 : 0) + lane]!;
      penalty = this.speedPenalty(edge.speeds_non_faded.no_flow);
      const lanes = edge.lane_count ?? 1;
      stressTerms = [
        { label: 'road', value: 1 },
        {
          label: 'class',
          value:
            (MAJOR_ROAD_CLASSES.includes(edge.road_class)
              ? road_class_factor
              : 1) *
            this.roadFactor *
            road_class_factors[ROAD_CLASSES.indexOf(edge.road_class)]!,
        },
        {
          label: 'lanes',
          value: lanes > 1 ? (lanes - 1) * 0.05 * this.roadFactor : 0,
        },
        { label: 'truck', value: edge.truck_route ? TRUCK_STRESS : 0 },
      ];
    }
    const network = edge.bicycle_network ? bicycle_network_factor : 1;

    const gradeIndex = Math.round(edge.weighted_grade * 0.6 + 6);
    const surface = SURFACES.indexOf(edge.surface);
    const terms = [
      { label: 'base', value: 1 },
      {
        label: 'grade',
        value: (1 - use_hills) * AVOID_HILLS_STRENGTH[gradeIndex]!,
      },
      ...stressTerms.map(({ label, value }) => ({
        label,
        value:
          accommodation * value * penalty * (label === 'class' ? 1 : network),
      })),
      {
        label: 'surface',
        value:
          surface >= FIRST_BAD_SURFACE
            ? avoid_bad_surfaces * SURFACE_FACTORS[surface - FIRST_BAD_SURFACE]!
            : 0,
      },
    ];
    return {
      factor: terms.reduce((sum, t) => sum + t.value, 0),
      terms,
      accommodation,
      network,
      speedPenalty: penalty,
      seconds: (edge.length * 1000 * 3.6) / edge.speed,
    };
  }
}
