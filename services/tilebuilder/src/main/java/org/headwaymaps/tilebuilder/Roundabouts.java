package org.headwaymaps.tilebuilder;

import com.onthegomap.planetiler.FeatureCollector;
import com.onthegomap.planetiler.ForwardingProfile;
import com.onthegomap.planetiler.VectorTile;
import com.onthegomap.planetiler.geo.GeoUtils;
import com.onthegomap.planetiler.geo.GeometryException;
import com.onthegomap.planetiler.reader.SourceFeature;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.IdentityHashMap;
import java.util.List;
import java.util.Map;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.CoordinateXY;
import org.locationtech.jts.geom.Geometry;
import org.locationtech.jts.geom.LineString;
import org.openmaptiles.generated.OpenMapTilesSchema.Transportation;

/**
 * Redraws roundabouts as circles, since tile coordinates are too coarse for a ring a few meters across to look round.
 */
class Roundabouts implements ForwardingProfile.LayerPostProcessor {

  static final String JUNCTION = "junction";
  /** The class of the center of the circle a roundabout runs around. */
  static final String ROUNDABOUT_CENTER = "roundabout_center";
  /** A roundabout's radius, in z14 tile pixels. */
  static final String RADIUS = "radius";

  /** How far a roundabout's mapped nodes may stray from a circle, as a share of its radius, to be drawn as one. */
  private static final double ROUNDNESS = 0.15;
  private static final int CIRCLE_SEGMENTS = 64;

  @Override
  public String name() {
    return Transportation.LAYER_NAME;
  }

  /** Tags the transportation features of a roundabout with its junction, from z14. */
  static void tagJunctions(SourceFeature source, FeatureCollector features) {
    if (!source.hasTag("junction", "roundabout")) {
      return;
    }
    for (var feature : features) {
      if (Transportation.LAYER_NAME.equals(feature.getLayer())) {
        feature.setAttrWithMinzoom(JUNCTION, "roundabout", 14);
      }
    }
  }

  /**
   * Redraws each roundabout whose nodes lie on a circle as arcs of that circle, tagged with its radius, and adds the
   * circle's center with its radius.
   */
  @Override
  public List<VectorTile.Feature> postProcess(int zoom, List<VectorTile.Feature> items) {
    Map<VectorTile.Feature, VectorTile.Feature> rounded = new IdentityHashMap<>();
    List<VectorTile.Feature> centers = new ArrayList<>();
    for (var ring : rings(items)) {
      List<Coordinate> nodes = new ArrayList<>();
      for (var item : ring) {
        for (var line : lines(item)) {
          nodes.addAll(List.of(line));
        }
      }
      Circle circle = Circle.fit(nodes);
      if (circle == null) {
        continue;
      }
      var first = ring.getFirst();
      centers.add(new VectorTile.Feature(first.layer(), first.id(),
        VectorTile.encodeGeometry(GeoUtils.point(circle.center)),
        new HashMap<>(Map.of(Transportation.Fields.CLASS, ROUNDABOUT_CENTER, RADIUS, circle.z14Radius(zoom)))));
      for (var item : ring) {
        var arcs = item.copyWithNewGeometry(circle.arcs(lines(item)));
        arcs.tags().put(RADIUS, circle.z14Radius(zoom));
        rounded.put(item, arcs);
      }
    }
    List<VectorTile.Feature> result = new ArrayList<>(items.size() + centers.size());
    items.forEach(item -> result.add(rounded.getOrDefault(item, item)));
    result.addAll(centers);
    return result;
  }

  /** The roundabout features among {@code items}, grouped into rings by their shared ends. */
  private static List<List<VectorTile.Feature>> rings(List<VectorTile.Feature> items) {
    List<List<VectorTile.Feature>> rings = new ArrayList<>();
    Map<Coordinate, List<VectorTile.Feature>> ringByEnd = new HashMap<>();
    for (var item : items) {
      if (!"roundabout".equals(item.tags().get(JUNCTION))) {
        continue;
      }
      List<VectorTile.Feature> ring = null;
      for (var end : ends(item)) {
        var other = ringByEnd.get(end);
        if (other == null || other == ring) {
          continue;
        }
        if (ring == null) {
          ring = other;
        } else {
          ring.addAll(other);
          rings.remove(other);
          for (var moved : other) {
            for (var movedEnd : ends(moved)) {
              ringByEnd.put(movedEnd, ring);
            }
          }
        }
      }
      if (ring == null) {
        ring = new ArrayList<>();
        rings.add(ring);
      }
      ring.add(item);
      for (var end : ends(item)) {
        ringByEnd.put(end, ring);
      }
    }
    return rings;
  }

  private static List<Coordinate> ends(VectorTile.Feature item) {
    var lines = lines(item);
    return lines.isEmpty() ? List.of() : List.of(lines.getFirst()[0], lines.getLast()[lines.getLast().length - 1]);
  }

  /** The coordinates of each line in {@code item}, or none when it can't be decoded. */
  private static List<Coordinate[]> lines(VectorTile.Feature item) {
    List<Coordinate[]> lines = new ArrayList<>();
    try {
      Geometry geometry = item.geometry().decode();
      for (int i = 0; i < geometry.getNumGeometries(); i++) {
        lines.add(geometry.getGeometryN(i).getCoordinates());
      }
    } catch (GeometryException e) {
      // Left out of any ring.
    }
    return lines;
  }

  private record Circle(Coordinate center, double radius) {

    /** The circle through {@code nodes}, or null when they don't lie close to one. */
    static Circle fit(List<Coordinate> nodes) {
      if (nodes.size() < 3) {
        return null;
      }
      // Least squares for x² + y² + ax + by + c = 0, about the centroid for precision.
      double mx = nodes.stream().mapToDouble(n -> n.x).average().orElseThrow();
      double my = nodes.stream().mapToDouble(n -> n.y).average().orElseThrow();
      double suu = 0, svv = 0, suv = 0, suuu = 0, svvv = 0, suvv = 0, svuu = 0;
      for (var node : nodes) {
        double u = node.x - mx;
        double v = node.y - my;
        suu += u * u;
        svv += v * v;
        suv += u * v;
        suuu += u * u * u;
        svvv += v * v * v;
        suvv += u * v * v;
        svuu += v * u * u;
      }
      double determinant = suu * svv - suv * suv;
      if (Math.abs(determinant) < 1e-12) {
        return null;
      }
      double bu = (suuu + suvv) / 2;
      double bv = (svvv + svuu) / 2;
      double uc = (bu * svv - bv * suv) / determinant;
      double vc = (bv * suu - bu * suv) / determinant;
      var center = new CoordinateXY(uc + mx, vc + my);
      double radius = Math.sqrt(uc * uc + vc * vc + (suu + svv) / nodes.size());
      for (var node : nodes) {
        if (Math.abs(node.distance(center) - radius) > ROUNDNESS * radius) {
          return null;
        }
      }
      return new Circle(center, radius);
    }

    /** The radius in z14 tile pixels, from this tile's at {@code zoom}. */
    double z14Radius(int zoom) {
      return Math.round(radius / Math.pow(2, zoom - 14) * 100) / 100.0;
    }

    /** Each of {@code lines}, redrawn along this circle from where it starts to where it ends. */
    Geometry arcs(List<Coordinate[]> lines) {
      return GeoUtils.JTS_FACTORY.createMultiLineString(lines.stream().map(this::arc).toArray(LineString[]::new));
    }

    private LineString arc(Coordinate[] line) {
      double start = angle(line[0]);
      double sweep = 0;
      for (int i = 1; i < line.length; i++) {
        sweep += Math.IEEEremainder(angle(line[i]) - angle(line[i - 1]), 2 * Math.PI);
      }
      int segments = Math.max(1, (int) Math.ceil(Math.abs(sweep) / (2 * Math.PI) * CIRCLE_SEGMENTS));
      Coordinate[] arc = new Coordinate[segments + 1];
      for (int i = 0; i <= segments; i++) {
        double angle = start + sweep * i / segments;
        arc[i] = new CoordinateXY(center.x + radius * Math.cos(angle), center.y + radius * Math.sin(angle));
      }
      return GeoUtils.JTS_FACTORY.createLineString(arc);
    }

    private double angle(Coordinate point) {
      return Math.atan2(point.y - center.y, point.x - center.x);
    }
  }
}
