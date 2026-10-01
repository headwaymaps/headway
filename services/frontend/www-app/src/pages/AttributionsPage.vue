<template>
  <div class="top-card">
    <search-box
      :tabindex="1"
      @did-select-place="searchBoxDidSelectPlace"
      @did-submit-search="
        (searchText) =>
          $router.push(`/search/${encodeURIComponent(searchText)}`)
      "
    />
  </div>

  <div class="bottom-card attributions">
    <h1>{{ $t('attributions') }}</h1>
    <p>
      Headway is mostly other people's work, stitched together. We're grateful
      to everyone who builds and shares the open data and open source software
      that make it possible.
    </p>

    <section v-for="section in sections" :key="section.title">
      <h2>{{ section.title }}</h2>
      <ul>
        <li v-for="credit in section.credits" :key="credit.name">
          <a :href="credit.url" target="_blank" rel="noopener">{{
            credit.name
          }}</a>
          — {{ credit.thanks }}
        </li>
      </ul>
    </section>

    <section>
      <h2>And many more</h2>
      <p>
        Beyond these are hundreds of smaller libraries, tools, and data sets
        that each make something here a little nicer. Thank you to all of their
        authors and maintainers.
      </p>
    </section>
  </div>
</template>

<script lang="ts">
import { getBaseMap } from 'src/components/BaseMap.vue';
import SearchBox from 'src/components/SearchBox.vue';
import Place from 'src/models/Place';
import { defineComponent } from 'vue';

interface Credit {
  name: string;
  url: string;
  thanks: string;
}

interface Section {
  title: string;
  credits: Credit[];
}

const sections: Section[] = [
  {
    title: 'Map data',
    credits: [
      {
        name: 'OpenStreetMap',
        url: 'https://www.openstreetmap.org/copyright',
        thanks:
          'the roads, paths, buildings, and places on this map, built by millions of contributors. © OpenStreetMap contributors.',
      },
      {
        name: 'Natural Earth',
        url: 'https://www.naturalearthdata.com',
        thanks:
          'public domain coastlines and boundaries for the zoomed-out map.',
      },
      {
        name: "Who's On First",
        url: 'https://whosonfirst.org',
        thanks: 'the gazetteer of places behind search.',
      },
      {
        name: 'OpenAddresses',
        url: 'https://openaddresses.io',
        thanks: 'address points that help search find where you mean.',
      },
      {
        name: 'Transitland Atlas',
        url: 'https://github.com/transitland/transitland-atlas',
        thanks:
          'a catalog of transit feeds, and the transit agencies who publish their schedules and realtime data openly.',
      },
    ],
  },
  {
    title: 'Building and serving the map',
    credits: [
      {
        name: 'Planetiler',
        url: 'https://github.com/onthegomap/planetiler',
        thanks: 'turns the whole planet into map tiles.',
      },
      {
        name: 'OpenMapTiles',
        url: 'https://openmaptiles.org',
        thanks: 'the vector tile schema our map is drawn from.',
      },
      {
        name: 'Martin',
        url: 'https://martin.maplibre.org',
        thanks: 'serves the map tiles.',
      },
      {
        name: 'Valhalla',
        url: 'https://github.com/valhalla/valhalla',
        thanks: 'driving, biking, and walking directions.',
      },
      {
        name: 'OpenTripPlanner',
        url: 'https://www.opentripplanner.org',
        thanks: 'transit directions.',
      },
      {
        name: 'Pelias',
        url: 'https://pelias.io',
        thanks: 'search and reverse geocoding, with help from libpostal.',
      },
      {
        name: 'Dagger',
        url: 'https://dagger.io',
        thanks: 'runs the build that assembles all of the above.',
      },
    ],
  },
  {
    title: 'How the map looks',
    credits: [
      {
        name: 'OpenStreetMap Americana',
        url: 'https://github.com/osm-americana/openstreetmap-americana',
        thanks: 'highway shields, and a lot of inspiration.',
      },
      {
        name: 'OSM Liberty',
        url: 'https://github.com/maputnik/osm-liberty',
        thanks: 'the map style we started from.',
      },
      {
        name: 'Maki',
        url: 'https://github.com/mapbox/maki',
        thanks: 'points of interest icons.',
      },
      {
        name: 'Roboto and Overpass',
        url: 'https://fonts.google.com',
        thanks: 'the typefaces for map labels.',
      },
    ],
  },
  {
    title: 'This app',
    credits: [
      {
        name: 'MapLibre',
        url: 'https://maplibre.org',
        thanks: 'draws the map in your browser.',
      },
      {
        name: 'Vue',
        url: 'https://vuejs.org',
        thanks: 'and Quasar, which the interface is built with.',
      },
    ],
  },
];

export default defineComponent({
  name: 'AttributionsPage',
  components: { SearchBox },
  data: function () {
    return { sections };
  },
  mounted: function () {
    getBaseMap()?.removeAllMarkers();
  },
  methods: {
    searchBoxDidSelectPlace(place?: Place) {
      if (place) {
        this.$router.push(`/place/${place.urlEncodedId()}`);
      }
    },
  },
});
</script>

<style lang="scss">
.attributions {
  padding: 8px 16px 16px;

  h1 {
    font-size: 1.3rem;
    line-height: 1.4;
    margin: 0 0 8px;
  }

  h2 {
    font-size: 1rem;
    font-weight: 500;
    line-height: 1.4;
    margin: 16px 0 4px;
  }

  ul {
    margin: 0;
    padding-left: 20px;
  }

  li {
    margin: 4px 0;
  }
}
</style>
