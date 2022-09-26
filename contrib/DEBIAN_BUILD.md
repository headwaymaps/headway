# Headway installation on Debian

The [install_headway.debian.sh](./install_headway.debian.sh) script contributed by Santiago Crespo will automatically deploy Headway on your Debian server, using either one of the predefined extracts or your own. It should also work on Ubuntu but this has not been tested yet.

The default port is 8080. If you want to change it for another non-privileged port, edit the PORT variable at the beginning of the script.

This script will automatically:
 * create a "headway" user. If you want to change the user, edit the USER variable.
 * download and install everything required, including Docker, Earthly and Headway
 * add a systemd service that will run at boot
 * add scripts for the service to start, stop and reload. Note that reload upgrades the headway source, downloads fresh data and process everything from scratch.
 * edit /etc/crontab so there is a daily reload at a random time between 00:00 and 06:00

## Usage

Download the script and make it executable:

`wget https://raw.githubusercontent.com/headwaymaps/headway/main/contrib/install_headway.debian.sh`

`chmod +x install_headway.debian.sh`

### Option 1: using a predefined extract

If you want Headway to use one of the predefined extract from the list of supported cities, run:

`sudo ./install_headway.debian.sh PUBLIC_URL CityName`

Example:

`sudo ./install_headway.debian.sh "https://myheadway.site" Amsterdam`

Run the script without arguments to get the list of available cities.

### Option 2: using your own extract

Place your .osm.pbf file in the same directory as the script. The extension .osm.pbf is important! And the name of this file should not match any of the cities with predefined extracts. You'll need to specify a list of the comma separated country codes that your extract will cover.

`sudo ./install_headway.debian.sh PUBLIC_URL COUNTRY_CODES`

Example:

`sudo ./install_headway.debian.sh "https://myheadway.site" NL`

Edit /usr/local/bin/upgrade_and_recreate_headway.sh and uncomment and modify the commented line, if you want to enable the automatic daily download of your fresh custom pbf extract.

## If the installation fails

Please [create an issue](https://github.com/headwaymaps/headway/issues/new) or send a PR if you can fix it! 😁

## After the installation

After the install finishes successfully, it is possible to check the status, stop, start and restart the Headway service:

`sudo service headway status`

`sudo service headway stop`

`sudo service headway start`

`sudo service headway restart`

But doing a reload will download and build the latest Headway, get fresh data and reprocess everything:
`sudo service headway reload`
