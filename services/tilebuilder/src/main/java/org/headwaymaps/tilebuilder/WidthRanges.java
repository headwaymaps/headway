package org.headwaymaps.tilebuilder;

import com.onthegomap.planetiler.VectorTile;
import com.onthegomap.planetiler.geo.GeoUtils;
import com.onthegomap.planetiler.geo.GeometryException;
import com.onthegomap.planetiler.geo.GeometryType;
import com.onthegomap.planetiler.geo.TileCoord;
import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.Comparator;
import java.util.Deque;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.SortedSet;
import java.util.TreeSet;
import java.util.stream.Stream;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.Envelope;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.LineSegment;
import org.openmaptiles.generated.OpenMapTilesSchema.Transportation;

/**
 * Joins each run of street pieces that differ only in width into one line, carrying its widths as ranges along it, so
 * {@link RoadSurfaces} can taper a street from one width to the next.
 *
 * <p>The ranges are {@code start,end,width} triples, the bounds as fractions of the line's length,
 * a width of NaN where the style picks it. Between two ranges, the width eases from one to the next. Where a tile's
 * edge cuts a line short, a range can start or end past it, so the line's width eases as it would uncut.
 *
 * <p>Where a line ends against another street of its class, both ease to the average of their widths at the joint.
 * Where a street is mapped as a pair of oneways, each widens where the other runs close, so together they're as wide
 * as both until a median opens between them, and each tapers up to the full width of a wider line it joins.
 */
class WidthRanges {

  static final String WIDTH_RANGES = "width_ranges";
  /** How long a taper is, per meter the width changes. */
  static final double TAPER_PER_METER_OF_CHANGE = 6;
  private static final double MIN_TAPER_METERS = 8;
  /** How long a taper to or from a width the style picks is, since its size isn't known here. */
  private static final double UNKNOWN_TAPER_METERS = 15;
  /** How far apart to sample a width that varies with the distance to another line. */
  private static final double SAMPLE_METERS = 1;
  /** How much a sampled width may vary, in meters, and still be one range. */
  private static final double SAMPLE_TOLERANCE = 0.05;
  /** How closely two lines must run in opposite directions to be a pair of oneways, as the cosine of their angle. */
  private static final double OPPOSING = -0.7;
  /** How sharply a line may turn back where two pieces meet and still be joined, as the cosine of the turn. */
  private static final double HAIRPIN = -0.7;
  /** How sharply a street may turn where it meets another and still carry on as it, as the cosine of the turn. */
  private static final double CARRIES_ON = 0.5;

  private static final double EARTH_CIRCUMFERENCE_METERS = 40_075_016.686;
  private static final double TILE_UNITS = 256;

  /** The size of a tile unit in meters, at the tile's middle. */
  static double metersPerUnit(TileCoord tile) {
    double n = Math.pow(2, tile.z());
    double latitude = Math.atan(Math.sinh(Math.PI * (1 - 2 * (tile.y() + 0.5) / n)));
    return EARTH_CIRCUMFERENCE_METERS * Math.cos(latitude) / n / TILE_UNITS;
  }

  static List<VectorTile.Feature> join(List<VectorTile.Feature> items, double metersPerUnit)
    throws GeometryException {
    List<VectorTile.Feature> result = new ArrayList<>();
    Map<Map<String, Object>, List<Piece>> groups = new HashMap<>();
    for (var item : items) {
      if (item.geometry().geomType() != GeometryType.LINE || item.tags().containsKey(Roundabouts.RADIUS)) {
        result.add(withWidthRanges(item));
        continue;
      }
      var tags = new HashMap<>(item.tags());
      Double width = (Double) tags.remove(HeadwayProfile.WIDTH);
      Geometry geometry = item.geometry().decode();
      for (int i = 0; i < geometry.getNumGeometries(); i++) {
        groups.computeIfAbsent(tags, k -> new ArrayList<>())
          .add(new Piece(item, geometry.getGeometryN(i).getCoordinates(), width));
      }
    }
    var unmapped = byEnd(groups);
    groups.values().forEach(pieces -> pieces.replaceAll(piece -> piece.withWidthOfNeighbors(unmapped)));
    var byEnd = byEnd(groups);
    Map<Chain, Map<String, Object>> chains = new HashMap<>();
    for (var group : groups.entrySet()) {
      for (var chain : chains(group.getValue(), isOneway(group.getKey()), byEnd)) {
        chains.put(chain, group.getKey());
      }
    }
    var oneways = chains.keySet().stream().filter(Chain::oneway).toList();
    for (var entry : chains.entrySet()) {
      var chain = entry.getKey();
      var ranges = chain.withAbutting(byEnd).ranges(metersPerUnit);
      if (chain.oneway()) {
        ranges = chain.widenedBeside(oneways, ranges, chain.ranges(metersPerUnit), metersPerUnit, byEnd);
      }
      result.add(chain.feature(entry.getValue(), ranges));
    }
    return result;
  }

  private static Map<Coordinate, List<Piece>> byEnd(Map<Map<String, Object>, List<Piece>> groups) {
    Map<Coordinate, List<Piece>> byEnd = new HashMap<>();
    for (var piece : groups.values().stream().flatMap(List::stream).toList()) {
      byEnd.computeIfAbsent(piece.first(), k -> new ArrayList<>()).add(piece);
      byEnd.computeIfAbsent(piece.last(), k -> new ArrayList<>()).add(piece);
    }
    return byEnd;
  }

  /** A feature this doesn't join, with its width as a single range. */
  private static VectorTile.Feature withWidthRanges(VectorTile.Feature item) {
    if (!(item.tags().get(HeadwayProfile.WIDTH) instanceof Double width)) {
      return item;
    }
    var tags = new HashMap<>(item.tags());
    tags.remove(HeadwayProfile.WIDTH);
    tags.put(WIDTH_RANGES, format(List.of(new Range(0, 1, width))));
    return new VectorTile.Feature(item.layer(), item.id(), item.geometry(), tags, item.group());
  }

  private static boolean isOneway(Map<String, Object> tags) {
    return tags.get(Transportation.Fields.ONEWAY) instanceof Number oneway && oneway.intValue() != 0;
  }

  /**
   * The runs of {@code pieces} that meet end to end where no other piece of their class, in {@code allByEnd}, does. A
   * oneway run only joins a piece's end to the next one's start.
   */
  static List<Chain> chains(List<Piece> pieces, boolean oneway, Map<Coordinate, List<Piece>> allByEnd) {
    Map<Coordinate, List<Piece>> byEnd = new HashMap<>();
    for (var piece : pieces) {
      byEnd.computeIfAbsent(piece.first(), k -> new ArrayList<>()).add(piece);
      byEnd.computeIfAbsent(piece.last(), k -> new ArrayList<>()).add(piece);
    }
    Set<Piece> used = new HashSet<>();
    List<Chain> chains = new ArrayList<>();
    for (var piece : pieces) {
      if (!used.add(piece)) {
        continue;
      }
      Deque<Piece> chain = new ArrayDeque<>(List.of(piece));
      for (Piece next; (next = continuation(chain.getLast(), chain.getLast().last(), byEnd, allByEnd, used, oneway,
        true)) != null; ) {
        chain.addLast(next);
      }
      for (Piece previous; (previous = continuation(chain.getFirst(), chain.getFirst().first(), byEnd, allByEnd, used,
        oneway, false)) != null; ) {
        chain.addFirst(previous);
      }
      chains.add(new Chain(List.copyOf(chain)));
    }
    return chains;
  }

  /**
   * The unused piece that continues past {@code end} of {@code piece}, turned to run on from it, or null where the line
   * ends or forks.
   */
  private static Piece continuation(Piece piece, Coordinate end, Map<Coordinate, List<Piece>> byEnd,
    Map<Coordinate, List<Piece>> allByEnd, Set<Piece> used, boolean oneway, boolean forward) {
    var meeting = byEnd.get(end);
    if (meeting.size() != 2 || allByEnd.get(end).stream()
      .filter(other -> Objects.equals(other.streetClass(), piece.streetClass())).count() != 2) {
      return null;
    }
    var other = meeting.get(0) == piece ? meeting.get(1) : meeting.get(0);
    if (other == piece || used.contains(other)) {
      return null;
    }
    boolean runsOn = forward ? other.first().equals(end) : other.last().equals(end);
    if (!runsOn && oneway) {
      return null;
    }
    var next = runsOn ? other : other.reversed();
    if (forward ? turnsBack(piece, next) : turnsBack(next, piece)) {
      return null;
    }
    used.add(other);
    return next;
  }

  /** Whether {@code other} runs on from {@code piece} where both end at {@code end}, rather than crossing it. */
  private static boolean carriesOn(Piece piece, Piece other, Coordinate end) {
    var in = piece.directionAt(end, true);
    var out = other.directionAt(end, false);
    return in[0] * out[0] + in[1] * out[1] > CARRIES_ON;
  }

  /** Whether {@code to} doubles back from where {@code from} ends, like the two halves of a dual carriageway. */
  private static boolean turnsBack(Piece from, Piece to) {
    var a = from.coords();
    var b = to.coords();
    var in = unit(a[a.length - 2], a[a.length - 1]);
    var out = unit(b[0], b[1]);
    return in[0] * out[0] + in[1] * out[1] < HAIRPIN;
  }

  /** One line of a feature, or where {@code taperFully}, a street a chain abuts, which it tapers all the way to. */
  record Piece(VectorTile.Feature feature, Coordinate[] coords, Double width, boolean taperFully) {

    Piece(VectorTile.Feature feature, Coordinate[] coords, Double width) {
      this(feature, coords, width, false);
    }

    Coordinate first() {
      return coords[0];
    }

    Coordinate last() {
      return coords[coords.length - 1];
    }

    Piece reversed() {
      var reversed = coords.clone();
      Collections.reverse(Arrays.asList(reversed));
      return new Piece(feature, reversed, width, taperFully);
    }

    double length() {
      double length = 0;
      for (int i = 1; i < coords.length; i++) {
        length += coords[i - 1].distance(coords[i]);
      }
      return length;
    }

    /** This, or where its width isn't mapped, as wide as the narrowest street of its class it meets end to end. */
    Piece withWidthOfNeighbors(Map<Coordinate, List<Piece>> byEnd) {
      if (width != null) {
        return this;
      }
      var narrowest = Stream.of(first(), last())
        .flatMap(end -> byEnd.getOrDefault(end, List.of()).stream())
        .filter(other -> other.width() != null && Objects.equals(other.streetClass(), streetClass()))
        .mapToDouble(Piece::width)
        .min();
      return narrowest.isPresent() ? new Piece(feature, coords, narrowest.getAsDouble(), taperFully) : this;
    }

    double widthOrNaN() {
      return width == null ? Double.NaN : width;
    }

    /** Its direction where it ends at {@code end}, heading into it, or else heading away from it. */
    double[] directionAt(Coordinate end, boolean into) {
      boolean atLast = coords[coords.length - 1].equals(end);
      var tip = atLast ? coords[coords.length - 1] : coords[0];
      var inner = atLast ? coords[coords.length - 2] : coords[1];
      return into ? unit(inner, tip) : unit(tip, inner);
    }

    Object streetClass() {
      return feature.tags().get(Transportation.Fields.CLASS);
    }

    boolean oneway() {
      return isOneway(feature.tags());
    }

    /** 1 where traffic runs the way the line is drawn, -1 where it runs against it. */
    int travelSign() {
      return feature.tags().get(Transportation.Fields.ONEWAY) instanceof Number oneway && oneway.intValue() < 0 ? -1 : 1;
    }
  }

  /** A stretch of a oneway and its width. */
  private record Segment(Coordinate from, Coordinate to, double width) {

    double distance(Coordinate point) {
      return new LineSegment(from, to).distance(point);
    }
  }

  private static double[] unit(Coordinate from, Coordinate to) {
    double length = from.distance(to);
    return length == 0 ? new double[]{0, 0} : new double[]{(to.x - from.x) / length, (to.y - from.y) / length};
  }

  record Range(double start, double end, double value) {}

  record Chain(List<Piece> pieces) {

    VectorTile.Feature feature(Map<String, Object> groupTags, List<Range> ranges) {
      var first = pieces.getFirst().feature();
      var tags = new HashMap<>(groupTags);
      if (ranges.stream().anyMatch(range -> !Double.isNaN(range.value()))) {
        tags.put(WIDTH_RANGES, format(ranges));
      }
      var coords = coords();
      var line = GeoUtils.JTS_FACTORY.createLineString(coords);
      return new VectorTile.Feature(first.layer(), first.id(), VectorTile.encodeGeometry(line), tags, first.group());
    }

    Coordinate[] coords() {
      List<Coordinate> coords = new ArrayList<>(List.of(pieces.getFirst().coords()));
      for (var piece : pieces.subList(1, pieces.size())) {
        coords.addAll(List.of(piece.coords()).subList(1, piece.coords().length));
      }
      return coords.toArray(Coordinate[]::new);
    }

    boolean oneway() {
      return pieces.getFirst().oneway();
    }

    /** The width of the piece each segment of {@link #coords()} belongs to. */
    private double[] segmentWidths() {
      List<Double> widths = new ArrayList<>();
      for (var piece : pieces) {
        for (int i = 1; i < piece.coords().length; i++) {
          widths.add(piece.widthOrNaN());
        }
      }
      return widths.stream().mapToDouble(Double::doubleValue).toArray();
    }

    /** The way traffic runs from one end of this chain to the other. */
    double[] travelHeading() {
      var coords = coords();
      var heading = unit(coords[0], coords[coords.length - 1]);
      int sign = pieces.getFirst().travelSign();
      return new double[]{heading[0] * sign, heading[1] * sign};
    }

    /** This chain's stretches, with each piece's width. */
    List<Segment> segments() {
      List<Segment> segments = new ArrayList<>();
      for (var piece : pieces) {
        if (piece.width() == null) {
          continue;
        }
        var coords = piece.coords();
        for (int i = 1; i < coords.length; i++) {
          segments.add(new Segment(coords[i - 1], coords[i], piece.width()));
        }
      }
      return segments;
    }

    /** The line of its class this oneway and another split from at {@code end}, or null where they don't. */
    private Piece splitFrom(Coordinate end, Map<Coordinate, List<Piece>> byEnd) {
      var endPiece = end.equals(pieces.getFirst().first()) ? pieces.getFirst() : pieces.getLast();
      var others = byEnd.getOrDefault(end, List.of()).stream()
        .filter(other -> pieces.stream().noneMatch(piece -> piece.feature() == other.feature()))
        .filter(other -> Objects.equals(other.streetClass(), endPiece.streetClass()) && other.width() != null)
        .toList();
      if (others.stream().noneMatch(Piece::oneway)) {
        return null;
      }
      return others.stream().filter(other -> !other.oneway() && carriesOn(endPiece, other, end))
        .max(Comparator.comparingDouble(Piece::width)).orElse(null);
    }

    /** How far {@code point} is from the centerline of {@code line}, carried on straight past {@code end}. */
    private static double offsetFrom(Piece line, Coordinate end, Coordinate point) {
      var next = line.first().equals(end) ? line.coords()[1] : line.coords()[line.coords().length - 2];
      double dx = next.x - end.x;
      double dy = next.y - end.y;
      return Math.abs(dx * (point.y - end.y) - dy * (point.x - end.x)) / Math.hypot(dx, dy);
    }

    /**
     * {@code ranges} widened where an opposing oneway of its class runs close, so that together they're as wide as
     * both until a median opens between them. Near where the two split from a line, they're together no wider than it:
     * neither is wider than its {@code plain} ranges or the line, less how far apart the two run.
     */
    List<Range> widenedBeside(List<Chain> oneways, List<Range> ranges, List<Range> plain, double metersPerUnit,
      Map<Coordinate, List<Piece>> byEnd) {
      var streetClass = pieces.getFirst().streetClass();
      var coords = coords();
      var reach = new Envelope();
      Arrays.stream(coords).forEach(reach::expandToInclude);
      var heading = travelHeading();
      List<Segment> partners = new ArrayList<>();
      for (var other : oneways) {
        if (other == this || !Objects.equals(other.pieces.getFirst().streetClass(), streetClass)) {
          continue;
        }
        var otherHeading = other.travelHeading();
        if (heading[0] * otherHeading[0] + heading[1] * otherHeading[1] >= OPPOSING) {
          continue;
        }
        for (var segment : other.segments()) {
          var near = new Envelope(reach);
          near.expandBy(segment.width() / metersPerUnit);
          if (near.intersects(segment.from(), segment.to())) {
            partners.add(segment);
          }
        }
      }
      if (partners.isEmpty()) {
        return ranges;
      }

      double[] ownWidths = segmentWidths();
      var startLine = splitFrom(coords[0], byEnd);
      var endLine = splitFrom(coords[coords.length - 1], byEnd);
      double startSplit = startLine == null ? Double.NaN : startLine.width();
      double endSplit = endLine == null ? Double.NaN : endLine.width();
      double length = pieces.stream().mapToDouble(Piece::length).sum();
      int samples = Math.max(1, (int) Math.ceil(length * metersPerUnit / SAMPLE_METERS));
      double[] widths = new double[samples + 1];
      boolean widened = false;
      int segment = 0;
      double segmentStart = 0;
      for (int k = 0; k <= samples; k++) {
        double along = length * k / samples;
        while (segment + 2 < coords.length && segmentStart + coords[segment].distance(coords[segment + 1]) < along) {
          segmentStart += coords[segment].distance(coords[segment + 1]);
          segment++;
        }
        var from = coords[segment];
        var to = coords[segment + 1];
        double t = from.distance(to) == 0 ? 0 : (along - segmentStart) / from.distance(to);
        var point = new Coordinate(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
        double base = valueAt(ranges, length == 0 ? 0 : along / length);
        double split = Double.isNaN(startSplit) ? endSplit
          : Double.isNaN(endSplit) ? startSplit
          : startSplit + (endSplit - startSplit) * along / length;
        double width = base;
        double apart = Double.POSITIVE_INFINITY;
        for (var partner : partners) {
          double distance = partner.distance(point) * metersPerUnit;
          apart = Math.min(apart, distance);
          width = Math.max(width, ownWidths[segment] + partner.width() - distance);
        }
        if (!Double.isNaN(split)) {
          // Each half's outer edge stays within the line's, even where the halves aren't yet side by side.
          boolean nearerStart = endLine == null || startLine != null && along < length / 2;
          double offset = nearerStart ? offsetFrom(startLine, coords[0], point)
            : offsetFrom(endLine, coords[coords.length - 1], point);
          apart = Math.max(apart, 2 * offset * metersPerUnit);
          width = Math.min(base, Math.max(valueAt(plain, length == 0 ? 0 : along / length), split - apart));
        }
        widths[k] = width;
        widened |= Math.abs(width - base) > SAMPLE_TOLERANCE;
      }
      if (!widened) {
        return ranges;
      }

      // A renderer eases between knots, which can overshoot the line between them, so knot wherever it misses.
      var knots = new TreeSet<>(knots(widths));
      while (true) {
        var result = rangesThrough(knots, widths, ranges);
        int worst = worstMiss(result, widths);
        if (worst < 0) {
          return result;
        }
        knots.add(worst);
      }
    }

    /** {@code ranges} past either end of the line, and one of no length at each knot of {@code widths} between. */
    private static List<Range> rangesThrough(SortedSet<Integer> knots, double[] widths, List<Range> ranges) {
      int samples = widths.length - 1;
      List<Range> result = new ArrayList<>();
      ranges.stream().filter(range -> range.end() < 0).forEach(range -> add(result, range));
      double previous = Double.NaN;
      for (int k : knots) {
        // Knots within the tolerance of the last are one range, so the width holds between them.
        double width = Math.abs(widths[k] - previous) < SAMPLE_TOLERANCE ? previous : widths[k];
        add(result, new Range((double) k / samples, (double) k / samples, width));
        previous = width;
      }
      ranges.stream().filter(range -> range.start() > 1).forEach(range -> add(result, range));
      return result;
    }

    /** The sample of {@code widths} that {@code ranges} eases furthest from, or -1 where none is beyond tolerance. */
    private static int worstMiss(List<Range> ranges, double[] widths) {
      int worst = -1;
      double worstMiss = SAMPLE_TOLERANCE;
      for (int k = 0; k < widths.length; k++) {
        double miss = Math.abs(valueAt(ranges, (double) k / (widths.length - 1)) - widths[k]);
        if (miss > worstMiss) {
          worstMiss = miss;
          worst = k;
        }
      }
      return worst;
    }

    /** The fewest samples of {@code widths} that lines between them miss no sample by more than the tolerance. */
    private static List<Integer> knots(double[] widths) {
      var knots = new TreeSet<>(List.of(0, widths.length - 1));
      Deque<int[]> spans = new ArrayDeque<>(List.of(new int[]{0, widths.length - 1}));
      while (!spans.isEmpty()) {
        var span = spans.pop();
        int a = span[0];
        int b = span[1];
        int worst = -1;
        double worstMiss = SAMPLE_TOLERANCE;
        for (int k = a + 1; k < b; k++) {
          double line = widths[a] + (widths[b] - widths[a]) * (k - a) / (b - a);
          if (Math.abs(widths[k] - line) > worstMiss) {
            worstMiss = Math.abs(widths[k] - line);
            worst = k;
          }
        }
        if (worst >= 0) {
          knots.add(worst);
          spans.push(new int[]{a, worst});
          spans.push(new int[]{worst, b});
        }
      }
      return List.copyOf(knots);
    }

    /**
     * This chain with a piece of no length at each end that another street of its class carries on as, so the line
     * tapers toward it: a oneway all the way up to a wider line it joins, anything else halfway to the widest.
     */
    Chain withAbutting(Map<Coordinate, List<Piece>> byEnd) {
      List<Piece> extended = new ArrayList<>(pieces);
      var before = abutting(pieces.getFirst().first(), byEnd);
      var after = abutting(pieces.getLast().last(), byEnd);
      if (before != null) {
        extended.addFirst(before);
      }
      if (after != null) {
        extended.addLast(after);
      }
      return new Chain(extended);
    }

    private Piece abutting(Coordinate end, Map<Coordinate, List<Piece>> byEnd) {
      var streetClass = pieces.getFirst().streetClass();
      var endPiece = end.equals(pieces.getFirst().first()) ? pieces.getFirst() : pieces.getLast();
      var others = byEnd.getOrDefault(end, List.of()).stream()
        .filter(other -> pieces.stream().noneMatch(piece -> piece.feature() == other.feature()))
        .filter(other -> Objects.equals(other.streetClass(), streetClass) && other.width() != null)
        .filter(other -> carriesOn(endPiece, other, end))
        .toList();
      if (oneway()) {
        var joined = others.stream().filter(other -> !other.oneway() && other.width() > endPiece.widthOrNaN())
          .max(Comparator.comparingDouble(Piece::width));
        if (joined.isPresent()) {
          return new Piece(joined.get().feature(), new Coordinate[]{end}, joined.get().width(), true);
        }
      } else if (others.stream().anyMatch(Piece::oneway)) {
        // The oneways it splits into widen all the way to it.
        return null;
      }
      return others.stream()
        .max(Comparator.comparingDouble(Piece::width))
        .map(widest -> new Piece(widest.feature(), new Coordinate[]{end}, widest.width()))
        .orElse(null);
    }

    /** Each piece's width over its share of the line, leaving room around each change to taper. */
    List<Range> ranges(double metersPerUnit) {
      int n = pieces.size();
      double[] taperBefore = new double[n];
      double[] taperAfter = new double[n];
      for (int i = 0; i + 1 < n; i++) {
        var piece = pieces.get(i);
        var next = pieces.get(i + 1);
        if (Double.compare(piece.widthOrNaN(), next.widthOrNaN()) != 0) {
          // Not limited by the pieces' lengths here, which a tile's edge cuts short, so neighboring tiles agree on
          // where a taper runs.
          double taper = taperMeters(piece.widthOrNaN(), next.widthOrNaN()) / metersPerUnit;
          if (next.taperFully()) {
            // Full width from a round end cap's radius out, so the wider line's cap is covered.
            double hold = next.width() / 2 / metersPerUnit;
            taperAfter[i] = taper + hold;
            taperBefore[i + 1] = -hold;
          } else if (piece.taperFully()) {
            double hold = piece.width() / 2 / metersPerUnit;
            taperAfter[i] = -hold;
            taperBefore[i + 1] = taper + hold;
          } else {
            taperAfter[i] = taper / 2;
            taperBefore[i + 1] = taper / 2;
          }
        }
      }
      double total = pieces.stream().mapToDouble(Piece::length).sum();
      List<Range> ranges = new ArrayList<>();
      double start = 0;
      for (int i = 0; i < n; i++) {
        var piece = pieces.get(i);
        double end = start + piece.length();
        // A tile's edge can cut a taper short, leaving a range that starts or ends past the line.
        double rangeStart = start + taperBefore[i];
        double rangeEnd = end - taperAfter[i];
        if (rangeStart > rangeEnd) {
          if (i == n - 1) {
            rangeEnd = rangeStart;
          } else if (i == 0) {
            rangeStart = rangeEnd;
          } else {
            // Too short for its tapers to reach its width, so the width eases past it.
            start = end;
            continue;
          }
        }
        add(ranges, new Range(rangeStart / total, rangeEnd / total, piece.widthOrNaN()));
        start = end;
      }
      return ranges;
    }

    /** Adds {@code range}, extending the last range instead when it has the same value. */
    private static void add(List<Range> ranges, Range range) {
      if (!ranges.isEmpty() && Double.compare(ranges.getLast().value(), range.value()) == 0) {
        ranges.set(ranges.size() - 1, new Range(ranges.getLast().start(), range.end(), range.value()));
      } else {
        ranges.add(range);
      }
    }
  }

  /**
   * The value of {@code ranges} at {@code fraction} of the way along its line, as a renderer eases it: along a monotone
   * cubic across each gap, level where a range has length, and through a range of no length at the slope of its
   * neighbors.
   */
  static double valueAt(List<Range> ranges, double fraction) {
    for (int i = 0; i < ranges.size(); i++) {
      var range = ranges.get(i);
      if (fraction <= range.end()) {
        if (fraction >= range.start() || i == 0) {
          return range.value();
        }
        double gapStart = ranges.get(i - 1).end();
        double gap = range.start() - gapStart;
        double x = (fraction - gapStart) / gap;
        double from = ranges.get(i - 1).value();
        double to = range.value();
        double fromSlope = slopeAt(ranges, i - 1) * gap;
        double toSlope = slopeAt(ranges, i) * gap;
        return (2 * x * x * x - 3 * x * x + 1) * from + (x * x * x - 2 * x * x + x) * fromSlope +
          (-2 * x * x * x + 3 * x * x) * to + (x * x * x - x * x) * toSlope;
      }
    }
    return ranges.getLast().value();
  }

  /** The slope of the eased value through range {@code k}: none where it has length, else the Fritsch-Butland mean. */
  private static double slopeAt(List<Range> ranges, int k) {
    if (k == 0 || k == ranges.size() - 1 || ranges.get(k).start() != ranges.get(k).end()) {
      return 0;
    }
    double before = ranges.get(k).start() - ranges.get(k - 1).end();
    double after = ranges.get(k + 1).start() - ranges.get(k).end();
    if (before <= 0 || after <= 0) {
      return 0;
    }
    double in = (ranges.get(k).value() - ranges.get(k - 1).value()) / before;
    double out = (ranges.get(k + 1).value() - ranges.get(k).value()) / after;
    return in * out > 0 ? 2 / (1 / in + 1 / out) : 0;
  }

  static double taperMeters(double from, double to) {
    if (Double.isNaN(from) || Double.isNaN(to)) {
      return UNKNOWN_TAPER_METERS;
    }
    return Math.max(MIN_TAPER_METERS, TAPER_PER_METER_OF_CHANGE * Math.abs(from - to));
  }

  /** The ranges {@link #format} wrote. */
  static List<Range> parse(String text) {
    var numbers = Arrays.stream(text.split(",")).mapToDouble(Double::parseDouble).toArray();
    List<Range> ranges = new ArrayList<>();
    for (int i = 0; i + 2 < numbers.length; i += 3) {
      ranges.add(new Range(numbers[i], numbers[i + 1], numbers[i + 2]));
    }
    return ranges;
  }

  static String format(List<Range> ranges) {
    var text = new StringBuilder();
    for (var range : ranges) {
      if (!text.isEmpty()) {
        text.append(',');
      }
      text.append(round(range.start())).append(',').append(round(range.end())).append(',')
        .append(Double.isNaN(range.value()) ? "NaN" : round(range.value()));
    }
    return text.toString();
  }

  private static String round(double value) {
    return Double.toString(Math.round(value * 10_000) / 10_000.0);
  }
}
