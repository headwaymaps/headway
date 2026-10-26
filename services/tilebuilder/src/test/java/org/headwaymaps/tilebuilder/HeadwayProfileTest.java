package org.headwaymaps.tilebuilder;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;

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

  @ParameterizedTest
  @CsvSource(nullValues = "-", value = {
    "7, 7.0",
    "7.3, 7.5",
    "7 m, 7.0",
    "24', 7.5",
    "0, -",
    "wide, -",
  })
  void parsesWidths(String mapped, Double meters) {
    if (meters == null) {
      assertNull(HeadwayProfile.width(mapped));
    } else {
      assertEquals(meters, HeadwayProfile.width(mapped));
    }
  }

  @ParameterizedTest
  @CsvSource(nullValues = "-", value = {
    "primary, 7, 4, 7.0",
    "primary, -, 4, 13.0",
    "primary, -, 5, 16.5",
    "primary, -, -, -",
    "footway, -, 2, -",
  })
  void widthsFallBackToLanes(String highway, String mapped, String lanes, Double meters) {
    Map<String, Object> tags = new HashMap<>(Map.of("highway", highway));
    if (mapped != null) {
      tags.put("width", mapped);
    }
    if (lanes != null) {
      tags.put("lanes", lanes);
    }
    var line = GeoUtils.JTS_FACTORY.createLineString(GeoUtils.coordinateSequence(0, 0, 1, 1));
    assertEquals(meters, HeadwayProfile.width(SimpleFeature.create(line, tags)));
  }
}
