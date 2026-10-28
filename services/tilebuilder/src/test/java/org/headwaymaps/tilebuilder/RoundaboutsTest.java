package org.headwaymaps.tilebuilder;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.onthegomap.planetiler.VectorTile;
import com.onthegomap.planetiler.geo.GeoUtils;
import com.onthegomap.planetiler.geo.GeometryException;
import java.util.Arrays;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.geom.Coordinate;
import org.locationtech.jts.geom.Geometry;

class RoundaboutsTest {

  private static VectorTile.Feature line(Map<String, Object> tags, double... xys) {
    var geometry = GeoUtils.JTS_FACTORY.createLineString(GeoUtils.coordinateSequence(xys));
    return new VectorTile.Feature("transportation", 1, VectorTile.encodeGeometry(geometry), new HashMap<>(tags));
  }

  private static Geometry decode(VectorTile.Feature feature) throws GeometryException {
    return feature.geometry().decode();
  }

  @Test
  void roundsRoundabouts() throws GeometryException {
    // A radius-10 ring mapped as an octagon in two halves, and a street leading to it.
    double[] octagon = new double[18];
    for (int i = 0; i <= 8; i++) {
      octagon[2 * i] = 100 + 10 * Math.cos(Math.PI * i / 4);
      octagon[2 * i + 1] = 100 + 10 * Math.sin(Math.PI * i / 4);
    }
    Map<String, Object> roundabout = Map.of("class", "minor", "junction", "roundabout");
    var first = line(roundabout, Arrays.copyOfRange(octagon, 0, 10));
    var second = line(roundabout, Arrays.copyOfRange(octagon, 8, 18));
    var street = line(Map.of("class", "minor"), 110, 100, 150, 100);

    var result = new Roundabouts().postProcess(14, List.of(first, second, street));

    assertEquals(street, result.get(2));
    var center = result.get(3);
    assertEquals("roundabout_center", center.tags().get("class"));
    assertEquals(10, (double) center.tags().get("radius"), 0.1);
    assertEquals(new Coordinate(100, 100), decode(center).getCoordinate());
    for (var arc : result.subList(0, 2)) {
      assertEquals(10, (double) arc.tags().get("radius"), 0.1);
      Coordinate[] points = decode(arc).getCoordinates();
      assertEquals(33, points.length);
      for (var point : points) {
        assertEquals(10, point.distance(new Coordinate(100, 100)), 0.1);
      }
    }
  }

  @Test
  void leavesLopsidedRingsAsMapped() {
    Map<String, Object> roundabout = Map.of("class", "minor", "junction", "roundabout");
    var ring = line(roundabout, 0, 0, 20, 0, 40, 0, 40, 10, 20, 10, 0, 10, 0, 0);

    assertEquals(List.of(ring), new Roundabouts().postProcess(14, List.of(ring)));
  }
}
