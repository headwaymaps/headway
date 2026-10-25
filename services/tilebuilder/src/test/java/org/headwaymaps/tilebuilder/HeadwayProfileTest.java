package org.headwaymaps.tilebuilder;

import static org.junit.jupiter.api.Assertions.assertEquals;

import com.onthegomap.planetiler.geo.GeoUtils;
import com.onthegomap.planetiler.reader.SimpleFeature;
import java.util.HashMap;
import java.util.Map;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.CsvSource;

class HeadwayProfileTest {

  @ParameterizedTest
  @CsvSource(nullValues = "-", value = {
    "crossing, marked, -, false",
    "crossing, traffic_signals, -, false",
    "crossing, uncontrolled, zebra, false",
    "crossing, marked, no, true",
    "crossing, uncontrolled, no, true",
    "crossing, unmarked, -, true",
    "crossing, -, -, true",
    "crossing, -, lines, false",
    "sidewalk, -, -, false",
  })
  void unmarkedCrossings(String footway, String crossing, String markings, boolean unmarked) {
    Map<String, Object> tags = new HashMap<>(Map.of("highway", "footway", "footway", footway));
    if (crossing != null) {
      tags.put("crossing", crossing);
    }
    if (markings != null) {
      tags.put("crossing:markings", markings);
    }
    var line = GeoUtils.JTS_FACTORY.createLineString(GeoUtils.coordinateSequence(0, 0, 1, 1));
    assertEquals(unmarked, HeadwayProfile.isUnmarkedCrossing(SimpleFeature.create(line, tags)));
  }
}
