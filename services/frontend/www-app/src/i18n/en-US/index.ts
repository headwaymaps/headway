export default {
  routing_area_not_supported:
    'Sorry, directions are not supported for this trip area.',
  transit_routing_area_not_supported:
    'Sorry, transit directions are not currently supported for this trip area.',
  transit_area_not_supported_for_source:
    'Sorry, trips starting here are outside of our current transit coverage area.',
  transit_area_not_supported_for_destination:
    'Sorry, this destination is outside of our current transit coverage area.',
  transit_trip_error_unknown:
    'Sorry, unable to find transit directions for this trip.',
  transit_routing_not_enabled:
    'Transit directions are disabled or incorrectly configured. Please contact the server administrator',
  routing_error_unknown:
    'Sorry, there was an unknown error while trying to get directions for this trip.',
  other_routing_error_with_$message:
    'Unable to get directions for this trip — {message}',
  try_driving_directions: 'Try driving directions instead?',
  where_to_question: 'Where to?',
  my_location: 'My Location',
  location_permission_denied_banner:
    'Location permission was denied. Check your privacy settings.',
  dropped_pin: 'Dropped Pin',
  via_$place: 'via {place}',
  via$transit_route: 'via route {transitRoute}',
  times: {
    $n_seconds: '{n} seconds',
    $n_minute: '{n} minute',
    $n_minutes: '{n} minutes',
    $n_day: '{n} day',
    $n_days: '{n} days',
    $n_hour: '{n} hour',
    $n_hours: '{n} hours',
  },
  times_shortform: {
    $n_seconds: '{n} sec',
    $n_minute: '{n} min',
    $n_minutes: '{n} min',
    $n_day: '{n} day',
    $n_days: '{n} day',
    $n_hour: '{n} hr',
    $n_hours: '{n} hr',
  },
  time_range$startTime$endTime: '{startTime} - {endTime}',
  go_home: 'Go Home',
  oops_nothing_here: 'Oops. Nothing here...',
  search: {
    from: 'From',
    to: 'To',
  },
  modes: {
    transit: 'Transit',
    drive: 'Drive',
    bike: 'Bike',
    walk: 'Walk',
  },
  punctuation_list_seperator: ', ',
  shortened_distances: {
    kilometers: 'km',
    miles: 'mi',
    meters: 'm',
  },
  walk_distance: '{preformattedDistance} walk total',
  bike_distance: '{preformattedDistance} bike total',
  route_picker_show_route_details_btn: 'Details',
  trip_search_depart_at: 'Leave at',
  trip_search_arrive_by: 'Arrive by',
  trip_search_depart_now: 'Leave now',
  trip_search_transit_with_bike: '🚲 Transit with a bike',
  departs_$timeDuration_from_now: 'in {timeDuration}',
  departs_$timeDuration_since_now: '{timeDuration} ago',
  departs_at_$location: 'from {location}',
  transit_timeline_wait_for_transit_$timeDuration: 'wait up to {timeDuration}',
  edit_poi_button: 'Edit Details',
  edit_poi_on_osm_button: 'Edit on OpenStreetMap',
  edit_poi_about_osm:
    'This data is from OpenStreetMap, a community maintained mapping project. You can edit OpenStreetMap, and your edits will eventually be reflected here.',
  opening_hours_is_open: 'Open',
  opening_hours_is_open_until_$time: 'Open until {time}',
  opening_hours_is_open_until_later_$day_$time: 'Open until {time} {day}',
  opening_hours_is_open_until_tomorrow_$time: 'Open until {time} tomorrow',
  opening_hours_is_closed: 'Closed',
  opening_hours_is_closed_until_$time: 'Closed until {time}',
  opening_hours_is_closed_until_later_$day_$time: 'Closed until {time} {day}',
  opening_hours_is_closed_until_tomorrow_$time: 'Closed until {time} tomorrow',
  opening_hours_show_more_times: 'Show hours',
  opening_hours_hide_more_times: 'Hide hours',
  search_results_not_found_header: 'No results found. 😢',
  search_results_not_found_subheader:
    'Something missing? Consider adding it to {osmLink} so it can eventually appear here.',
};
