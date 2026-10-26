package org.headwaymaps.tilebuilder;

import com.onthegomap.planetiler.FeatureCollector;
import com.onthegomap.planetiler.Planetiler;
import com.onthegomap.planetiler.reader.SourceFeature;
import com.onthegomap.planetiler.util.ZoomFunction;
import com.onthegomap.planetiler.util.Parse;
import org.openmaptiles.OpenMapTilesProfile;
import org.openmaptiles.generated.OpenMapTilesSchema.Transportation;

/** OpenMapTiles, plus what Headway's style draws beyond it. */
public class HeadwayProfile extends OpenMapTilesProfile {

  static final String FOOTWAY = "footway";
  static final String CROSSING = "crossing";
  static final String UNMARKED = "unmarked";
  static final String WIDTH = "width";
  /** A typical city lane, in meters. */
  static final double LANE_WIDTH = 3.25;
  /**
   * How far past a tile's edge its streets reach, in pixels of 256: about 100m at z15, so a tile sees the neighbors
   * that shape a street's width as the next tile does.
   */
  private static final int NEIGHBOR_BUFFER_PIXELS = 32;

  public HeadwayProfile(Planetiler runner) {
    super(runner);
    registerHandler(new Roundabouts());
  }

  @Override
  public void processFeature(SourceFeature source, FeatureCollector features) {
    super.processFeature(source, features);
    Roundabouts.tagJunctions(source, features);
    tagFootways(source, features);
    tagWidth(source, features);
  }

  /**
   * Tags the transportation features of sidewalks and crossings with their footway, and unmarked crossings as such, from
   * z14.
   */
  static void tagFootways(SourceFeature source, FeatureCollector features) {
    if (!source.hasTag(FOOTWAY, "sidewalk", CROSSING)) {
      return;
    }
    boolean unmarked = isUnmarkedCrossing(source);
    for (var feature : features) {
      if (Transportation.LAYER_NAME.equals(feature.getLayer())) {
        feature.setAttrWithMinzoom(FOOTWAY, source.getTag(FOOTWAY), 14);
        if (unmarked) {
          feature.setAttrWithMinzoom(CROSSING, UNMARKED, 14);
        }
      }
    }
  }

  /**
   * Tags the transportation features of a street with its width in meters, from z14: as mapped, or else from its lanes,
   * rounded to the half meter.
   */
  static void tagWidth(SourceFeature source, FeatureCollector features) {
    Double width = width(source);
    if (width == null) {
      return;
    }
    for (var feature : features) {
      if (Transportation.LAYER_NAME.equals(feature.getLayer())) {
        feature.setAttrWithMinzoom(WIDTH, width, 14)
          .setBufferPixelOverrides(ZoomFunction.minZoom(14, NEIGHBOR_BUFFER_PIXELS));
      }
    }
  }

  /** A street's mapped width, or else the width of its lanes. */
  static Double width(SourceFeature source) {
    Double mapped = width(source.getTag(WIDTH));
    if (mapped != null || source.hasTag("highway", "footway", "cycleway", "path", "pedestrian", "steps", "bridleway")) {
      return mapped;
    }
    Integer lanes = Parse.parseIntOrNull(source.getTag("lanes"));
    return lanes == null || lanes < 1 || lanes > 12 ? null : Math.round(lanes * LANE_WIDTH * 2) / 2.0;
  }

  static Double width(Object value) {
    Double meters = Parse.meters(value);
    return meters == null || meters <= 0 || meters > 100 ? null : Math.round(meters * 2) / 2.0;
  }

  /** A crossing with nothing mapped to say it's painted on the road. */
  static boolean isUnmarkedCrossing(SourceFeature source) {
    if (!source.hasTag(FOOTWAY, CROSSING)) {
      return false;
    }
    Object markings = source.getTag("crossing:markings");
    if (markings != null) {
      return "no".equals(markings);
    }
    return !source.hasTag("crossing", "marked", "zebra", "uncontrolled", "traffic_signals");
  }
}
