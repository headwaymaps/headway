package org.headwaymaps.tilebuilder;

import com.onthegomap.planetiler.Planetiler;
import org.openmaptiles.OpenMapTilesProfile;

/** OpenMapTiles, plus what Headway's style draws beyond it. */
public class HeadwayProfile extends OpenMapTilesProfile {

  public HeadwayProfile(Planetiler runner) {
    super(runner);
  }
}
