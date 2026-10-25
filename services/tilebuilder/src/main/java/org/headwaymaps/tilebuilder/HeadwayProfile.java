package org.headwaymaps.tilebuilder;

import com.onthegomap.planetiler.FeatureCollector;
import com.onthegomap.planetiler.Planetiler;
import com.onthegomap.planetiler.reader.SourceFeature;
import org.openmaptiles.OpenMapTilesProfile;

/** OpenMapTiles, plus what Headway's style draws beyond it. */
public class HeadwayProfile extends OpenMapTilesProfile {

  static final String FOOTWAY = "footway";

  public HeadwayProfile(Planetiler runner) {
    super(runner);
    registerHandler(new Roundabouts());
  }

  @Override
  public void processFeature(SourceFeature source, FeatureCollector features) {
    if (isUnmarkedCrossing(source)) {
      return;
    }
    super.processFeature(source, features);
    Roundabouts.tagJunctions(source, features);
  }

  /** A crossing with nothing mapped to say it's painted on the road. */
  static boolean isUnmarkedCrossing(SourceFeature source) {
    if (!source.hasTag(FOOTWAY, "crossing")) {
      return false;
    }
    Object markings = source.getTag("crossing:markings");
    if (markings != null) {
      return "no".equals(markings);
    }
    return !source.hasTag("crossing", "marked", "zebra", "uncontrolled", "traffic_signals");
  }
}
