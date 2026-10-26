package org.headwaymaps.tilebuilder;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.onthegomap.planetiler.VectorTile;
import com.onthegomap.planetiler.geo.GeoUtils;
import com.onthegomap.planetiler.geo.GeometryException;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.stream.Collectors;
import org.junit.jupiter.api.Test;
import org.openmaptiles.generated.OpenMapTilesSchema.Transportation;

class WidthRangesTest {

  /** One tile unit per meter, so tapers are easy to read off. */
  private static final double METERS_PER_UNIT = 1;

  private static VectorTile.Feature street(Double width, Integer oneway, double... coords) {
    Map<String, Object> tags = new HashMap<>(Map.of(Transportation.Fields.CLASS, "primary"));
    if (width != null) {
      tags.put(HeadwayProfile.WIDTH, width);
    }
    if (oneway != null) {
      tags.put(Transportation.Fields.ONEWAY, oneway);
    }
    var line = GeoUtils.JTS_FACTORY.createLineString(GeoUtils.coordinateSequence(coords));
    return new VectorTile.Feature(Transportation.LAYER_NAME, 1, VectorTile.encodeGeometry(line), tags);
  }

  private static List<VectorTile.Feature> join(VectorTile.Feature... streets) throws GeometryException {
    return WidthRanges.join(List.of(streets), METERS_PER_UNIT);
  }

  @Test
  void joinsAWidthChangeAndTapersAcrossIt() throws GeometryException {
    var joined = join(street(10.0, null, 0, 0, 100, 0), street(7.0, null, 100, 0, 200, 0));
    assertEquals(1, joined.size());
    // A 3m change tapers over 18m, split across the two pieces.
    assertEquals("0.0,0.455,10.0,0.545,1.0,7.0", joined.getFirst().tags().get(WidthRanges.WIDTH_RANGES));
    assertEquals(3, joined.getFirst().geometry().decode().getNumPoints());
  }

  @Test
  void turnsAPieceAroundToJoinIt() throws GeometryException {
    var joined = join(street(10.0, null, 0, 0, 100, 0), street(7.0, null, 200, 0, 100, 0));
    assertEquals("0.0,0.455,10.0,0.545,1.0,7.0", joined.getFirst().tags().get(WidthRanges.WIDTH_RANGES));
  }

  @Test
  void takesAnUnmappedWidthFromTheStreetItMeets() throws GeometryException {
    var joined = join(street(10.0, null, 0, 0, 100, 0), street(null, null, 100, 0, 200, 0));
    assertEquals("0.0,1.0,10.0", joined.getFirst().tags().get(WidthRanges.WIDTH_RANGES));
  }

  @Test
  void leavesTheStyleToPickAWidthNothingNearbyMaps() throws GeometryException {
    var joined = join(street(10.0, null, 0, 0, 100, 0), street(null, null, 300, 0, 200, 0));
    assertTrue(joined.stream().anyMatch(feature -> !feature.tags().containsKey(WidthRanges.WIDTH_RANGES)));
  }

  @Test
  void doesNotJoinOnewaysThatMeetHeadOn() throws GeometryException {
    assertEquals(2, join(street(10.0, 1, 0, 0, 100, 0), street(7.0, 1, 200, 0, 100, 0)).size());
  }

  @Test
  void doesNotJoinAcrossAFork() throws GeometryException {
    var joined = join(
      street(10.0, null, 0, 0, 100, 0),
      street(7.0, null, 100, 0, 200, 0),
      street(7.0, null, 100, 0, 100, 100));
    assertEquals(3, joined.size());
  }

  @Test
  void meetsAnAbuttingStreetHalfway() throws GeometryException {
    var wide = street(40.0, null, 0, 0, 1000, 0);
    var narrow = street(20.0, null, 1000, 0, 2000, 0);
    narrow.tags().put(Transportation.Fields.BRUNNEL, "bridge");
    var joined = join(wide, narrow);
    // A 20m change tapers over 120m, half on each side of the joint, where both are 30m wide.
    assertEquals(
      Set.of("0.0,0.94,40.0,1.06,1.06,20.0", "-0.06,-0.06,40.0,0.06,1.0,20.0"),
      joined.stream().map(feature -> feature.tags().get(WidthRanges.WIDTH_RANGES)).collect(Collectors.toSet()));
  }

  @Test
  void leavesStreetsOfAnotherClassAlone() throws GeometryException {
    var minor = street(5.0, null, 100, 0, 200, 0);
    minor.tags().put(Transportation.Fields.CLASS, "minor");
    assertEquals(
      Set.of("0.0,1.0,10.0", "0.0,1.0,5.0"),
      join(street(10.0, null, 0, 0, 100, 0), minor).stream()
        .map(feature -> feature.tags().get(WidthRanges.WIDTH_RANGES)).collect(Collectors.toSet()));
  }

  @Test
  void tapersPastTheEndOfALineTheTileCutShort() throws GeometryException {
    var joined = join(street(10.0, null, 0, 0, 100, 0), street(7.0, null, 100, 0, 103, 0));
    // The taper ends 9m past the joint, but the line only runs on 3m.
    assertEquals("0.0,0.8835,10.0,1.0583,1.0583,7.0", joined.getFirst().tags().get(WidthRanges.WIDTH_RANGES));
  }

  @Test
  void widensAPairOfOnewaysWhereTheyRunClose() throws GeometryException {
    // Opposing oneways 4m apart, then 20m apart once a median opens.
    var east = street(6.5, 1, 0, 0, 50, 0, 60, 8, 100, 8);
    var west = street(6.5, 1, 100, -12, 60, -12, 50, -4, 0, -4);
    // Where they run 4m apart, each spans 9m, so together they're 13m curb to curb.
    assertEquals(
      Set.of(
        "0.0,0.4854,9.0,0.4951,0.4951,8.381,0.5049,0.5049,7.6033,0.5146,0.5146,6.7625,0.5243,1.0,6.5",
        "0.0,0.4757,6.5,0.4854,0.4854,6.7625,0.4951,0.4951,7.6033,0.5049,0.5049,8.381,0.5146,1.0,9.0"),
      join(east, west).stream()
        .map(feature -> feature.tags().get(WidthRanges.WIDTH_RANGES)).collect(Collectors.toSet()));
  }

  @Test
  void widensAPairToTheLineItSplitsFrom() throws GeometryException {
    var joined = join(
      street(16.5, null, 0, 0, 1000, 0),
      street(6.5, 1, 1000, 0, 2000, 200),
      street(6.5, 1, 2000, -200, 1000, 0));
    for (var feature : joined) {
      var ranges = (String) feature.tags().get(WidthRanges.WIDTH_RANGES);
      if (feature.tags().containsKey(Transportation.Fields.ONEWAY)) {
        // Together as wide as the line where they split, each back to its own width once they're 10m apart.
        assertTrue(Set.of(
          "0.0,0.0,16.5,0.001,0.001,16.1078,0.002,0.002,15.7157,0.0069,0.0069,13.7549,0.0186,0.0186,9.049,0.0225,0.0225,7.4804,0.0245,0.0245,6.6961,0.0255,1.0,6.5",
          "0.0,0.9745,6.5,0.9755,0.9755,6.6961,0.9775,0.9775,7.4804,0.9814,0.9814,9.049,0.9931,0.9931,13.7549,0.998,0.998,15.7157,0.999,0.999,16.1078,1.0,1.0,16.5").contains(ranges), ranges);
      } else {
        assertEquals("0.0,1.0,16.5", ranges);
      }
    }
  }

  @Test
  void doesNotJoinTheHalvesOfADualCarriagewayWhereTheyMeet() throws GeometryException {
    assertEquals(2, join(street(6.5, 1, 0, 0, 100, 0), street(6.5, 1, 100, 0, 0, 5)).size());
  }

  @Test
  void doesNotJoinWhereALineOfTheSameClassAlsoEnds() throws GeometryException {
    var joined = join(
      street(16.5, null, 0, 0, 100, 0),
      street(6.5, 1, 100, 0, 200, 100),
      street(6.5, 1, 200, -100, 100, 0));
    assertEquals(3, joined.size());
  }

  @Test
  void tapersALoneOnewayUpToTheLineItJoins() throws GeometryException {
    var joined = join(street(16.5, null, 0, 0, 1000, 0), street(6.5, 1, 1000, 0, 2000, 0));
    // A 10m change tapers over 60m, then holds 16.5m for the last 8.25m, the line's end cap's radius.
    assertEquals(
      Set.of("0.0,1.0,16.5", "0.0,0.0083,16.5,0.0683,1.0,6.5"),
      joined.stream().map(feature -> feature.tags().get(WidthRanges.WIDTH_RANGES)).collect(Collectors.toSet()));
  }

  @Test
  void widensAnEyeToTheLinesAtEitherEnd() throws GeometryException {
    var joined = join(
      street(13.0, null, 0, -100, 0, 0),
      street(6.5, 1, 0, 0, 4, 18, 0, 36),
      street(6.5, 1, 0, 36, -4, 18, 0, 0),
      street(16.5, null, 0, 36, 0, 136));
    // Each half is as wide as the line at either end, and between them, together as wide as both lines blended.
    assertEquals(
      Set.of("0.0,1.0,13.0", "0.0,1.0,16.5",
        "0.0,0.0,13.0,0.027,0.027,12.6622,0.1081,0.1081,11.6486,0.3514,0.3514,8.6081,0.4324,0.4324,7.5946,0.4595,0.4595,7.2568,0.4865,0.4865,6.9189,0.5135,0.5135,7.0135,0.5405,0.5405,7.5405,0.5676,0.5676,8.0676,0.6486,0.6486,9.6486,0.8919,0.8919,14.3919,0.973,0.973,15.973,1.0,1.0,16.5",
        "0.0,0.0,16.5,0.027,0.027,15.973,0.1081,0.1081,14.3919,0.3514,0.3514,9.6486,0.4324,0.4324,8.0676,0.4595,0.4595,7.5405,0.4865,0.4865,7.0135,0.5135,0.5135,6.9189,0.5405,0.5405,7.2568,0.6216,0.6216,8.2703,0.8649,0.8649,11.3108,0.9459,0.9459,12.3243,0.973,0.973,12.6622,1.0,1.0,13.0"),
      joined.stream().map(feature -> feature.tags().get(WidthRanges.WIDTH_RANGES)).collect(Collectors.toSet()));
  }
}
