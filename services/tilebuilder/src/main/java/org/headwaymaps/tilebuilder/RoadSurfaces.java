package org.headwaymaps.tilebuilder;

import com.onthegomap.planetiler.ForwardingProfile;
import com.onthegomap.planetiler.VectorTile;
import com.onthegomap.planetiler.geo.GeoUtils;
import com.onthegomap.planetiler.geo.GeometryException;
import com.onthegomap.planetiler.geo.GeometryType;
import com.onthegomap.planetiler.geo.TileCoord;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.stream.Collectors;
import org.locationtech.jts.algorithm.Orientation;
import org.locationtech.jts.algorithm.locate.IndexedPointInAreaLocator;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.Envelope;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.LineString;
import org.locationtech.jts.geom.Location;
import org.locationtech.jts.geom.Polygon;
import org.locationtech.jts.geom.PrecisionModel;
import org.locationtech.jts.geom.util.GeometryFixer;
import org.locationtech.jts.geom.util.PolygonExtracter;
import org.locationtech.jts.operation.buffer.BufferOp;
import org.locationtech.jts.operation.buffer.BufferParameters;
import org.locationtech.jts.operation.buffer.VariableBuffer;
import org.locationtech.jts.operation.distance.IndexedFacetDistance;
import org.locationtech.jts.densify.Densifier;
import org.locationtech.jts.operation.overlayng.OverlayNG;
import org.locationtech.jts.operation.overlayng.OverlayNGRobust;
import org.locationtech.jts.operation.union.UnaryUnionOp;
import org.locationtech.jts.precision.GeometryPrecisionReducer;
import org.locationtech.jts.simplify.VWSimplifier;
import org.openmaptiles.generated.OpenMapTilesSchema.Transportation;

/**
 * Draws the streets of the deepest tiles as the ground they cover: each street widened to its width, merged with the
 * streets of its grade, and smoothed, so that junctions, width changes and the halves of a dual carriageway come out as
 * one surface with curbs, rather than as strokes that must agree where they meet.
 */
class RoadSurfaces implements ForwardingProfile.TilePostProcessor {

  static final String LAYER_NAME = "street_surface";
  /** The ground between each street's curb and the far side of a sidewalk mapped beside it. */
  static final String SIDEWALK_LAYER_NAME = "sidewalk_surface";
  /** Marks a sidewalk's line once its ground is drawn beside the street instead. */
  static final String BESIDE_STREET = "beside_street";
  static final int ZOOM = 15;

  /** At most how wide a gap between streets is filled, and how round an inside corner is, as a radius in meters. */
  private static final double CLOSING_METERS = 5;
  /** How round a curb's outside corner is, and how thin a sliver is dropped, as a radius in meters. */
  private static final double OPENING_METERS = 1;
  /** At most how round a curb is where it turns a corner from one street into another, as a radius in meters. */
  private static final double CORNER_METERS = 5;
  /** How round a curb's corners are for the narrower street there, as a radius per meter of its width. */
  private static final double CORNER_PER_WIDTH = 0.4;
  /** How sharply a fill's edge can turn outward away from the streets before it's taken as a spike, in radians. */
  private static final double MAX_FILL_TURN = Math.toRadians(30);
  /** Every sidewalk's width, since where and how wide they're mapped is too unreliable to draw. */
  private static final double SIDEWALK_METERS = 2;
  /** How far a sidewalk can run from a curb and still be taken as that street's. */
  private static final double MAX_SIDEWALK_OFFSET_METERS = 12;
  /** How far past the curb into the street a sidewalk's ground is first laid, so none is missed along the curb. */
  private static final double INTO_STREET_METERS = 1;
  /** How far past a sidewalk's reach its ground is first laid, before it's cut to the curb offset. */
  private static final double PAST_REACH_METERS = 1;
  /** How round the land's corners are where sidewalks of different widths meet, as a radius in meters. */
  private static final double SIDEWALK_ROUNDING_METERS = 3;
  /** How squarely a sidewalk must face the curb nearest it to take its ground from that curb. */
  private static final double ACROSS_COSINE = 0.5;
  /** How far past a tile's edge its surfaces reach, in tile units, enough to hide the seam. */
  private static final double CLIP_UNITS = 2;
  /** How far a street's kind runs on along a street of another kind it meets, so the corners between them close. */
  private static final double STUB_METERS = 20;
  /** How far either way along a street its width is averaged over. */
  private static final double AVERAGE_METERS = 5;
  /** How far apart to sample a street's width along it. */
  private static final double SAMPLE_METERS = 2;
  /** The tile's own grid, 4096 to its 256 units, which surfaces are snapped to so that they overlay cleanly. */
  private static final PrecisionModel TILE_GRID = new PrecisionModel(4096 / 256.0);
  private static final BufferParameters FLAT_ENDS =
    new BufferParameters(8, BufferParameters.CAP_FLAT, BufferParameters.JOIN_ROUND, BufferParameters.DEFAULT_MITRE_LIMIT);

  /** The level streets run at, each level's streets drawn as one surface. */
  private enum Grade {
    TUNNEL, GROUND, BRIDGE;

    static Grade of(Object brunnel) {
      return "tunnel".equals(brunnel) ? TUNNEL : "bridge".equals(brunnel) ? BRIDGE : GROUND;
    }

    Map<String, Object> tags() {
      return this == GROUND ? Map.of() : Map.of(Transportation.Fields.BRUNNEL, name().toLowerCase());
    }
  }

  /** The kind of street a line is, whose gaps are closed only against streets of the same kind. */
  private record Kind(Object streetClass, Object ramp) {

    /** How far apart its streets can be and still close into one surface, as a radius in meters. */
    double closingMeters() {
      Double width = width(Map.of(Transportation.Fields.CLASS, String.valueOf(streetClass),
        Transportation.Fields.RAMP, ramp == null ? 0 : ramp));
      return width == null ? CLOSING_METERS : Math.min(CLOSING_METERS, CORNER_PER_WIDTH * width);
    }

    /** Whether this is a driveway, alley or the like, whose mouth a sidewalk runs straight across. */
    boolean isService() {
      return "service".equals(streetClass) || "track".equals(streetClass);
    }
  }

  /** One line of a street, and its width in tile units at each of its vertices. */
  private record Street(Grade grade, Kind kind, LineString line, double[] widths) {

    /**
     * {@code line}, with a vertex every sample along it, and its widths there from {@code ranges} in meters. A oneway's
     * widths aren't eased or averaged any wider, since a dual carriageway's half narrows as fast as it parts from the
     * other.
     */
    static Street of(Grade grade, Kind kind, LineString line, List<WidthRanges.Range> ranges, double fallback, boolean oneway,
      double metersPerUnit) {
      var dense = (LineString) Densifier.densify(line, SAMPLE_METERS / metersPerUnit);
      double total = dense.getLength();
      double[] widths = new double[dense.getNumPoints()];
      double along = 0;
      for (int i = 0; i < widths.length; i++) {
        if (i > 0) {
          along += dense.getCoordinateN(i - 1).distance(dense.getCoordinateN(i));
        }
        double width = ranges == null ? Double.NaN : WidthRanges.valueAt(ranges, total == 0 ? 0 : along / total);
        widths[i] = (Double.isNaN(width) ? fallback : width) / metersPerUnit;
      }
      var averaged = averaged(dense, widths, AVERAGE_METERS / metersPerUnit);
      if (oneway) {
        // At its ends a oneway keeps its mapped width, which is the line it splits from or joins, for others to taper to.
        Arrays.setAll(averaged, i -> i == 0 || i == widths.length - 1 ? widths[i] : Math.min(averaged[i], widths[i]));
        return new Street(grade, kind, dense, averaged);
      }
      return new Street(grade, kind, dense, eased(dense, averaged));
    }

    /** {@code widths}, each the mean of those within {@code reach} along the line, so the curbs don't wobble. */
    private static double[] averaged(LineString line, double[] widths, double reach) {
      double[] along = new double[widths.length];
      for (int i = 1; i < widths.length; i++) {
        along[i] = along[i - 1] + line.getCoordinateN(i - 1).distance(line.getCoordinateN(i));
      }
      double[] averaged = new double[widths.length];
      int low = 0;
      int high = 0;
      double sum = 0;
      for (int i = 0; i < widths.length; i++) {
        while (high < widths.length && along[high] <= along[i] + reach) {
          sum += widths[high++];
        }
        while (along[low] < along[i] - reach) {
          sum -= widths[low++];
        }
        averaged[i] = sum / (high - low);
      }
      return averaged;
    }

    /**
     * {@code widths}, widened wherever they fall faster than a taper would from a wider stretch, so a step in width
     * becomes a taper and never a notch.
     */
    private static double[] eased(LineString line, double[] widths) {
      double fall = 1 / WidthRanges.TAPER_PER_METER_OF_CHANGE;
      var eased = widths.clone();
      for (int i = 1; i < eased.length; i++) {
        double step = line.getCoordinateN(i - 1).distance(line.getCoordinateN(i));
        eased[i] = Math.max(eased[i], eased[i - 1] - fall * step);
      }
      for (int i = eased.length - 2; i >= 0; i--) {
        double step = line.getCoordinateN(i).distance(line.getCoordinateN(i + 1));
        eased[i] = Math.max(eased[i], eased[i + 1] - fall * step);
      }
      return eased;
    }

    /** This, narrowing to no wider than {@code width} at {@code end}, at the rate a taper would. */
    Street taperedTo(Coordinate end, double width) {
      boolean atFirst = end.equals2D(first());
      double fall = 1 / WidthRanges.TAPER_PER_METER_OF_CHANGE;
      var tapered = widths.clone();
      double along = 0;
      for (int k = 0; k < tapered.length; k++) {
        int i = atFirst ? k : tapered.length - 1 - k;
        if (k > 0) {
          int previous = atFirst ? i - 1 : i + 1;
          along += line.getCoordinateN(previous).distance(line.getCoordinateN(i));
        }
        double cap = width + fall * along;
        if (cap >= tapered[i]) {
          break;
        }
        tapered[i] = cap;
      }
      return new Street(grade, kind, line, tapered);
    }

    Coordinate first() {
      return line.getCoordinateN(0);
    }

    Coordinate last() {
      return line.getCoordinateN(line.getNumPoints() - 1);
    }

    double widthAt(Coordinate end) {
      return end.equals2D(first()) ? widths[0] : widths[widths.length - 1];
    }

    /** The ground this covers, flat across at either end. */
    Geometry widened() {
      double[] halves = Arrays.stream(widths).map(width -> width / 2).toArray();
      double widest = Arrays.stream(halves).max().orElseThrow();
      return new VariableBuffer(line, halves).getResult()
        .intersection(BufferOp.bufferOp(line, widest + 1, FLAT_ENDS));
    }

    /** The first {@code length} of this from its end at {@code end}, widened as it is along there. */
    Geometry stubFrom(Coordinate end, double length) {
      boolean atFirst = end.equals2D(first());
      List<Coordinate> coords = new ArrayList<>();
      List<Double> stubWidths = new ArrayList<>();
      double along = 0;
      for (int k = 0; k < widths.length; k++) {
        int i = atFirst ? k : widths.length - 1 - k;
        if (k > 0) {
          int previous = atFirst ? i - 1 : i + 1;
          double step = line.getCoordinateN(previous).distance(line.getCoordinateN(i));
          if (along + step > length) {
            double t = (length - along) / step;
            var from = line.getCoordinateN(previous);
            var to = line.getCoordinateN(i);
            coords.add(new Coordinate(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t));
            stubWidths.add(widths[previous] + (widths[i] - widths[previous]) * t);
            break;
          }
          along += step;
        }
        coords.add(line.getCoordinateN(i));
        stubWidths.add(widths[i]);
      }
      if (coords.size() < 2) {
        return GeoUtils.JTS_FACTORY.createPolygon();
      }
      var stub = GeoUtils.JTS_FACTORY.createLineString(coords.toArray(Coordinate[]::new));
      return new Street(grade, kind, stub, stubWidths.stream().mapToDouble(Double::doubleValue).toArray()).widened();
    }
  }

  /** Ground-level street surfaces, indexed to find the curb nearest a point. */
  private record Curbs(Geometry ground, IndexedFacetDistance toCurb, IndexedPointInAreaLocator onGround) {

    Curbs(Geometry ground) {
      this(ground, new IndexedFacetDistance(ground), new IndexedPointInAreaLocator(ground));
    }

    /** How far {@code line} runs from these curbs, as the median over its vertices. */
    double offset(Geometry line) {
      double[] offsets = Arrays.stream(line.getCoordinates())
        .mapToDouble(coordinate -> toCurb.distance(GeoUtils.JTS_FACTORY.createPoint(coordinate)))
        .sorted()
        .toArray();
      return offsets[offsets.length / 2];
    }
  }

  /** A ground-level sidewalk's line, the curbs it runs beside, and its median offset from them. */
  private record Sidewalk(Geometry line, Curbs curbs, double offset) {

    /**
     * {@code item} as a sidewalk beside a street's curb, or null if it isn't one. It's taken to run beside
     * {@code streets} rather than the driveways crossing it, unless it only runs beside driveways.
     */
    static Sidewalk of(VectorTile.Feature item, Curbs streets, Curbs all, double metersPerUnit)
      throws GeometryException {
      var tags = item.tags();
      if (!"sidewalk".equals(tags.get(HeadwayProfile.FOOTWAY)) || tags.get(Transportation.Fields.BRUNNEL) != null ||
        item.geometry().geomType() != GeometryType.LINE) {
        return null;
      }
      var decoded = item.geometry().decode();
      if (decoded.isEmpty()) {
        return null;
      }
      var line = Densifier.densify(decoded, SAMPLE_METERS / metersPerUnit);
      double max = MAX_SIDEWALK_OFFSET_METERS / metersPerUnit;
      var curbs = streets != null && streets.offset(line) <= max ? streets : all;
      double offset = curbs.offset(line);
      if (offset > max) {
        return null;
      }
      return new Sidewalk(line, curbs, offset);
    }

    /**
     * The ground within a sidewalk's width of the curb, beside it: the curb offset outward, so the sidewalk's far edge
     * follows the curb around corners and runs straight past driveways.
     */
    Geometry surface(double metersPerUnit) {
      double reach = SIDEWALK_METERS / metersPerUnit;
      double into = INTO_STREET_METERS / metersPerUnit;
      double past = reach + PAST_REACH_METERS / metersPerUnit;
      List<Geometry> beside = new ArrayList<>();
      for (int i = 0; i < line.getNumGeometries(); i++) {
        Coordinate[] coords = line.getGeometryN(i).getCoordinates();
        int n = coords.length;
        double[][] normals = new double[n][];
        Coordinate[] nearest = new Coordinate[n];
        double streetSide = 0;
        for (int k = 0; k < n; k++) {
          var before = coords[Math.max(k - 1, 0)];
          var after = coords[Math.min(k + 1, n - 1)];
          double length = Math.max(before.distance(after), 1e-9);
          normals[k] = new double[]{-(after.y - before.y) / length, (after.x - before.x) / length};
          nearest[k] = curbs.toCurb().nearestPoints(GeoUtils.JTS_FACTORY.createPoint(coords[k]))[0];
          streetSide += Math.signum(
            (nearest[k].x - coords[k].x) * normals[k][0] + (nearest[k].y - coords[k].y) * normals[k][1]);
        }
        double away = streetSide >= 0 ? -1 : 1;
        Coordinate[] inner = new Coordinate[n];
        Coordinate[] outer = new Coordinate[n];
        for (int k = 0; k < n; k++) {
          var from = coords[k];
          double distance = from.distance(nearest[k]);
          double ux = away * normals[k][0];
          double uy = away * normals[k][1];
          var foot = new Coordinate(from.x - ux * offset, from.y - uy * offset);
          if (distance > 0 && curbs.onGround().locate(from) == Location.EXTERIOR) {
            double nx = (from.x - nearest[k].x) / distance;
            double ny = (from.y - nearest[k].y) / distance;
            // Across from the curb nearest, unless that's a side street's rather than its own.
            if (nx * ux + ny * uy > ACROSS_COSINE) {
              ux = nx;
              uy = ny;
              foot = nearest[k];
            }
          }
          inner[k] = new Coordinate(foot.x - ux * into, foot.y - uy * into);
          outer[k] = new Coordinate(foot.x + ux * past, foot.y + uy * past);
        }
        for (int k = 1; k < n; k++) {
          var piece = GeoUtils.JTS_FACTORY.createMultiPointFromCoords(
            new Coordinate[]{inner[k - 1], inner[k], outer[k], outer[k - 1]}).convexHull();
          if (piece instanceof Polygon) {
            beside.add(piece);
          }
        }
      }
      var area = OverlayNGRobust.union(beside, GeoUtils.JTS_FACTORY);
      var near = area.getEnvelopeInternal();
      near.expandBy(2 * reach);
      var street = OverlayNGRobust.overlay(curbs.ground(), GeoUtils.JTS_FACTORY.toGeometry(near), OverlayNG.INTERSECTION);
      return OverlayNGRobust.overlay(street.buffer(reach), area, OverlayNG.INTERSECTION);
    }
  }

  private static final Envelope CLIPPED = new Envelope(-CLIP_UNITS, 256 + CLIP_UNITS, -CLIP_UNITS, 256 + CLIP_UNITS);

  @Override
  public Map<String, List<VectorTile.Feature>> postProcessTile(TileCoord tile,
    Map<String, List<VectorTile.Feature>> layers) throws GeometryException {
    var items = layers.get(Transportation.LAYER_NAME);
    if (tile.z() != ZOOM || items == null) {
      return layers;
    }
    double metersPerUnit = WidthRanges.metersPerUnit(tile);
    List<Street> streets = new ArrayList<>();
    Map<Coordinate, List<Street>> byEnd = new HashMap<>();
    for (var item : WidthRanges.join(items, metersPerUnit)) {
      var tags = item.tags();
      Double fallback = width(tags);
      if (item.geometry().geomType() != GeometryType.LINE || fallback == null) {
        continue;
      }
      var ranges = tags.get(WidthRanges.WIDTH_RANGES) instanceof String text ? WidthRanges.parse(text) : null;
      var grade = Grade.of(tags.get(Transportation.Fields.BRUNNEL));
      var kind = new Kind(tags.get(Transportation.Fields.CLASS), tags.get(Transportation.Fields.RAMP));
      boolean oneway = tags.get(Transportation.Fields.ONEWAY) instanceof Number direction && direction.intValue() != 0;
      var geometry = item.geometry().decode();
      for (int i = 0; i < geometry.getNumGeometries(); i++) {
        var street = Street.of(grade, kind, (LineString) geometry.getGeometryN(i), ranges, fallback, oneway, metersPerUnit);
        streets.add(street);
        for (var end : List.of(street.first(), street.last())) {
          byEnd.computeIfAbsent(end, k -> new ArrayList<>()).add(street);
        }
      }
    }
    List<Street> tapered = new ArrayList<>();
    for (var street : streets) {
      var narrowed = street;
      for (var end : List.of(street.first(), street.last())) {
        var widest = byEnd.get(end).stream()
          .filter(other -> other != street && other.grade() == street.grade())
          .mapToDouble(other -> other.widthAt(end))
          .max();
        if (widest.isPresent()) {
          narrowed = narrowed.taperedTo(end, widest.getAsDouble());
        }
      }
      tapered.add(narrowed);
    }
    double opening = OPENING_METERS / metersPerUnit;
    Map<Coordinate, List<Street>> taperedByEnd = new HashMap<>();
    for (var street : tapered) {
      for (var end : List.of(street.first(), street.last())) {
        taperedByEnd.computeIfAbsent(end, k -> new ArrayList<>()).add(street);
      }
    }
    var byGrade = tapered.stream().collect(Collectors.groupingBy(Street::grade));
    Map<Grade, Geometry> smoothed = new HashMap<>();
    Geometry streetGround = null;
    for (var entry : byGrade.entrySet()) {
      Map<Kind, List<Geometry>> widened = new HashMap<>();
      for (var street : entry.getValue()) {
        var own = widened.computeIfAbsent(street.kind(), k -> new ArrayList<>());
        own.add(street.widened());
        for (var end : List.of(street.first(), street.last())) {
          for (var other : taperedByEnd.get(end)) {
            if (other.grade() == street.grade() && !other.kind().equals(street.kind())) {
              own.add(other.stubFrom(end, STUB_METERS / metersPerUnit));
            }
          }
        }
      }
      Map<Kind, Geometry> kinds = new HashMap<>();
      for (var ofKind : widened.entrySet()) {
        var union = UnaryUnionOp.union(ofKind.getValue());
        double closing = ofKind.getKey().closingMeters() / metersPerUnit;
        var closed = union.buffer(closing).buffer(-closing);
        var spikes = GeoUtils.JTS_FACTORY.buildGeometry(fills(union, closed).get(true));
        kinds.put(ofKind.getKey(), GeometryFixer.fix(
          OverlayNGRobust.overlay(closed, spikes, OverlayNG.DIFFERENCE).buffer(-opening).buffer(opening)));
      }
      var surface = OverlayNGRobust.union(kinds.values(), GeoUtils.JTS_FACTORY);
      smoothed.put(entry.getKey(), snapped(GeometryFixer.fix(roundCorners(surface, entry.getValue(), metersPerUnit))));
      if (entry.getKey() == Grade.GROUND) {
        var streetKinds = kinds.entrySet().stream().filter(kind -> !kind.getKey().isService()).map(Map.Entry::getValue)
          .toList();
        if (!streetKinds.isEmpty()) {
          var streetsOnly = entry.getValue().stream().filter(street -> !street.kind().isService()).toList();
          streetGround = snapped(GeometryFixer.fix(
            roundCorners(OverlayNGRobust.union(streetKinds, GeoUtils.JTS_FACTORY), streetsOnly, metersPerUnit)));
        }
      }
    }
    var clip = GeoUtils.JTS_FACTORY.toGeometry(CLIPPED);
    List<VectorTile.Feature> surfaces = new ArrayList<>();
    for (var entry : smoothed.entrySet()) {
      var surface = polygonal(OverlayNGRobust.overlay(entry.getValue(), clip, OverlayNG.INTERSECTION));
      if (surface.isEmpty()) {
        continue;
      }
      surface.normalize();
      surfaces.add(new VectorTile.Feature(LAYER_NAME, 0, VectorTile.encodeGeometry(surface.reverse()),
        entry.getKey().tags()));
    }
    var ground = smoothed.get(Grade.GROUND);
    var allCurbs = ground == null ? null : new Curbs(ground);
    var streetCurbs = streetGround == null ? null : new Curbs(streetGround);
    List<VectorTile.Feature> transportation = new ArrayList<>();
    List<Geometry> sidewalks = new ArrayList<>();
    for (var item : items) {
      var sidewalk = allCurbs == null ? null : Sidewalk.of(item, streetCurbs, allCurbs, metersPerUnit);
      if (sidewalk == null) {
        transportation.add(item);
        continue;
      }
      sidewalks.add(sidewalk.surface(metersPerUnit));
      transportation.add(item.copyWithExtraAttrs(Map.of(BESIDE_STREET, 1)));
    }
    Map<String, List<VectorTile.Feature>> result = new HashMap<>(layers);
    result.put(Transportation.LAYER_NAME, transportation);
    result.put(LAYER_NAME, surfaces);
    if (!sidewalks.isEmpty()) {
      double rounding = SIDEWALK_ROUNDING_METERS / metersPerUnit;
      var beside = OverlayNGRobust.union(sidewalks, GeoUtils.JTS_FACTORY);
      if (streetGround != null) {
        beside = OverlayNGRobust.union(List.of(beside, streetGround), GeoUtils.JTS_FACTORY)
          .buffer(rounding).buffer(-rounding);
      }
      var sidewalk = OverlayNGRobust.overlay(beside, ground, OverlayNG.DIFFERENCE);
      sidewalk = polygonal(OverlayNGRobust.overlay(snapped(GeometryFixer.fix(sidewalk)), clip, OverlayNG.INTERSECTION));
      if (!sidewalk.isEmpty()) {
        sidewalk.normalize();
        result.put(SIDEWALK_LAYER_NAME, List.of(
          new VectorTile.Feature(SIDEWALK_LAYER_NAME, 0, VectorTile.encodeGeometry(sidewalk.reverse()), Map.of())));
      }
    }
    return result;
  }

  /**
   * {@code surface}, with the inside corners filled where its {@code streets} meet, rounder where the narrower street
   * there is wider. Only near junctions, so the gap between nearby streets stays open.
   */
  private static Geometry roundCorners(Geometry surface, List<Street> streets, double metersPerUnit) {
    Map<Coordinate, List<Double>> widths = new HashMap<>();
    for (var street : streets) {
      for (int i = 0; i < street.widths().length; i++) {
        widths.computeIfAbsent(street.line().getCoordinateN(i), k -> new ArrayList<>()).add(street.widths()[i]);
      }
    }
    List<Geometry> filleted = new ArrayList<>(List.of(surface));
    for (var vertex : widths.entrySet()) {
      if (vertex.getValue().size() < 2) {
        continue;
      }
      double narrowest = vertex.getValue().stream().mapToDouble(Double::doubleValue).min().orElseThrow();
      double widest = vertex.getValue().stream().mapToDouble(Double::doubleValue).max().orElseThrow();
      double radius = Math.min(CORNER_METERS / metersPerUnit, CORNER_PER_WIDTH * narrowest);
      var junction = GeoUtils.JTS_FACTORY.createPoint(vertex.getKey()).buffer(widest / 2 + 2 * radius);
      var near = OverlayNGRobust.overlay(surface, junction.buffer(2 * radius), OverlayNG.INTERSECTION);
      var closed = near.buffer(radius).buffer(-radius);
      fills(near, OverlayNGRobust.overlay(closed, junction, OverlayNG.INTERSECTION)).get(false).stream()
        .filter(fillet -> fillet.getArea() <= radius * radius)
        .forEach(filleted::add);
    }
    return OverlayNGRobust.union(filleted, GeoUtils.JTS_FACTORY);
  }


  /**
   * The pieces {@code closed} adds to {@code ground}, split by whether they come to a point away from it, as a closing
   * does on either side of a gap too narrow to pass yet too wide to bridge.
   */
  private static Map<Boolean, List<Polygon>> fills(Geometry ground, Geometry closed) {
    var toGround = new IndexedFacetDistance(ground);
    List<Polygon> pieces = PolygonExtracter.getPolygons(OverlayNGRobust.overlay(closed, ground, OverlayNG.DIFFERENCE));
    return pieces.stream().collect(Collectors.partitioningBy(piece -> isPointed(piece, toGround)));
  }

  /** Whether {@code piece}'s edge turns sharply outward anywhere off the ground {@code toGround} measures to. */
  private static boolean isPointed(Polygon piece, IndexedFacetDistance toGround) {
    var ring = piece.getExteriorRing().getCoordinates();
    double outward = Orientation.isCCW(ring) ? 1 : -1;
    for (int i = 0; i < ring.length - 1; i++) {
      var before = ring[i == 0 ? ring.length - 2 : i - 1];
      var at = ring[i];
      var after = ring[i + 1];
      double cross = (at.x - before.x) * (after.y - at.y) - (at.y - before.y) * (after.x - at.x);
      double dot = (at.x - before.x) * (after.x - at.x) + (at.y - before.y) * (after.y - at.y);
      if (Math.atan2(outward * cross, dot) > MAX_FILL_TURN &&
        toGround.distance(GeoUtils.JTS_FACTORY.createPoint(at)) > 1 / TILE_GRID.getScale()) {
        return true;
      }
    }
    return false;
  }

  /**
   * {@code surface} on the tile's grid, thinned first so rounding leaves no stair steps along its curves, or as it is in
   * the rare case snapping would make it invalid. Thinning goes by each vertex's neighbors, so neighboring tiles agree.
   */
  private static Geometry snapped(Geometry surface) {
    try {
      return GeometryPrecisionReducer.reduce(VWSimplifier.simplify(surface, 1 / TILE_GRID.getScale()), TILE_GRID);
    } catch (IllegalArgumentException e) {
      return surface;
    }
  }

  /** The areas of {@code geometry}, without the slivers an overlay can leave behind as lines or points. */
  private static Geometry polygonal(Geometry geometry) {
    return geometry.getFactory().buildGeometry(PolygonExtracter.getPolygons(geometry));
  }

  /** A street's width in meters as mapped, or else as the style draws its kind, or null for what isn't a street. */
  static Double width(Map<String, Object> tags) {
    if (tags.get(HeadwayProfile.WIDTH) instanceof Double width) {
      return width;
    }
    if ("roundabout".equals(tags.get(Roundabouts.JUNCTION))) {
      return 4.5;
    }
    boolean ramp = tags.get(Transportation.Fields.RAMP) instanceof Number r && r.intValue() == 1;
    var streetClass = String.valueOf(tags.get(Transportation.Fields.CLASS));
    if (ramp && !streetClass.equals("service") && !streetClass.equals("track")) {
      return 6.0;
    }
    return switch (streetClass) {
      case "service" -> switch (String.valueOf(tags.get(Transportation.Fields.SERVICE))) {
        case "driveway" -> 3.5;
        case "alley" -> 4.5;
        case "parking_aisle" -> 6.0;
        default -> 4.0;
      };
      case "track" -> 4.0;
      case "motorway" -> 11.0;
      case "trunk", "primary" -> 13.0;
      case "secondary" -> 12.0;
      case "tertiary" -> 10.0;
      case "minor" -> 8.0;
      default -> null;
    };
  }
}
