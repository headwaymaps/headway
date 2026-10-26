package org.headwaymaps.tilebuilder;

import com.onthegomap.planetiler.FeatureCollector;
import com.onthegomap.planetiler.Planetiler;
import com.onthegomap.planetiler.reader.SourceFeature;
import org.openmaptiles.OpenMapTilesProfile;
import org.openmaptiles.generated.OpenMapTilesSchema.Transportation;

/** OpenMapTiles, plus what Headway's style draws beyond it. */
public class HeadwayProfile extends OpenMapTilesProfile {

  static final String FOOTWAY = "footway";
  static final String CROSSING = "crossing";
  static final String UNMARKED = "unmarked";

  public HeadwayProfile(Planetiler runner) {
    super(runner);
    registerHandler(new Roundabouts());
  }

  @Override
  public void processFeature(SourceFeature source, FeatureCollector features) {
    super.processFeature(source, features);
    Roundabouts.tagJunctions(source, features);
    tagFootways(source, features);
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
