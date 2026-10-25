package org.headwaymaps.tilebuilder;

import com.onthegomap.planetiler.FeatureCollector;
import com.onthegomap.planetiler.Planetiler;
import com.onthegomap.planetiler.reader.SourceFeature;
import org.openmaptiles.OpenMapTilesProfile;

/** OpenMapTiles, plus what Headway's style draws beyond it. */
public class HeadwayProfile extends OpenMapTilesProfile {

  public HeadwayProfile(Planetiler runner) {
    super(runner);
    registerHandler(new Roundabouts());
  }

  @Override
  public void processFeature(SourceFeature source, FeatureCollector features) {
    super.processFeature(source, features);
    Roundabouts.tagJunctions(source, features);
  }
}
