#!/bin/bash
# Script to install Headway on Debian 11
# The default port is 8080. If you want to change it for another non-privileged port, edit the PORT variable below.
# This script will automatically:
# * create a "headway" user. If you want to change the user, edit the USER variable below.
# * download and install everything required, including Docker, Earthly and Headway
# * configure a systemd service set up to run at boot
# * add scripts for the service to start, stop and reload. Note that reload upgrades the headway source, downloads fresh data and process everything from scratch.
# * edit /etc/crontab so there is a daily reload at a random time between 00:00 and 06:00

# Minimum hardware (and suggested) for a large city:
# HD: 20GB (40GB+)
# CPU: 1 core? (8+ cores)
# RAM: 8GB? (20+)

set -e

USER=headway
PORT=8080
WAIT_SECONDS=20 # Seconds to wait before checking that the service is running

if [ $# -ne 2 ]; then
  echo "Use: $0 PUBLIC_URL [CityName] [COUNTRY_CODES]"
  echo ""
  echo "Example using a predefined extract: $0 \"https://myheadway.site\" Amsterdam"
  echo "The areas with predefined extracts available are: Aachen, Aarhus, Adelaide, Albuquerque, Alexandria, Amsterdam, Antwerpen, Arnhem, Auckland, Augsburg, Austin, Baghdad, Baku, Balaton, Bamberg, Bangkok, Barcelona, Basel, Beijing, Beirut, Berkeley, Berlin, Bern, Bielefeld, Birmingham, Bochum, Bogota, Bombay, Bonn, Bordeaux, Boulder, BrandenburgHavel, Braunschweig, Bremen, Bremerhaven, Brisbane, Bristol, Brno, Bruegge, Bruessel, Budapest, BuenosAires, Cairo, Calgary, Cambridge, CambridgeMa, Canberra, CapeTown, Chemnitz, Chicago, ClermontFerrand, Colmar, Copenhagen, Cork, Corsica, Corvallis, Cottbus, Cracow, CraterLake, Curitiba, Cusco, Dallas, Darmstadt, Davis, DenHaag, Denver, Dessau, Dortmund, Dresden, Dublin, Duesseldorf, Duisburg, Edinburgh, Eindhoven, Emden, Erfurt, Erlangen, Eugene, Flensburg, FortCollins, Frankfurt, FrankfurtOder, Freiburg, Gdansk, Genf, Gent, Gera, Glasgow, Gliwice, Goerlitz, Goeteborg, Goettingen, Graz, Groningen, Halifax, Halle, Hamburg, Hamm, Hannover, Heilbronn, Helsinki, Hertogenbosch, Huntsville, Innsbruck, Istanbul, Jena, Jerusalem, Johannesburg, Kaiserslautern, Karlsruhe, Kassel, Katowice, Kaunas, Kiel, Kiew, Koblenz, Koeln, Konstanz, LakeGarda, LaPaz, LaPlata, Lausanne, Leeds, Leipzig, Lima, Linz, Lisbon, Liverpool, Ljubljana, Lodz, London, Luebeck, Luxemburg, Lyon, Maastricht, Madison, Madrid, Magdeburg, Mainz, Malmoe, Manchester, Mannheim, Marseille, Melbourne, Memphis, MexicoCity, Miami, Minsk, Moenchengladbach, Montevideo, Montpellier, Montreal, Moscow, Muenchen, Muenster, NewDelhi, NewOrleans, NewYork, Nuernberg, Oldenburg, Oranienburg, Orlando, Oslo, Osnabrueck, Ostrava, Ottawa, Paderborn, Palma, PaloAlto, Paris, Perth, Philadelphia, PhnomPenh, Portland, PortlandME, Porto, PortoAlegre, Potsdam, Poznan, Prag, Providence, Regensburg, Riga, RiodeJaneiro, Rostock, Rotterdam, Ruegen, Saarbruecken, Sacramento, Saigon, Salzburg, SanFrancisco, SanJose, SanktPetersburg, SantaBarbara, SantaCruz, Santiago, Sarajewo, Schwerin, Seattle, Seoul, Sheffield, Singapore, Sofia, Stockholm, Stockton, Strassburg, Stuttgart, Sucre, Sydney, Szczecin, Tallinn, Tehran, Tilburg, Tokyo, Toronto, Toulouse, Trondheim, Tucson, Turin, UlanBator, Ulm, Usedom, Utrecht, Vancouver, Victoria, WarenMueritz, Warsaw, WashingtonDC, Waterloo, Wien, Wroclaw, Wuerzburg, Wuppertal, Zagreb, Zuerich"
  echo ""
  echo "To use your custom extract instead, place the .osm.pbf file in the current directory and replace NL with a list of comma separated country codes that your extract will cover."
  echo "Example using a custom extract: $0 \"https://myheadway.site\" NL"
  exit 1
fi

if [ "$EUID" -ne 0 ]; then
  echo "This scripts needs to be run as root. Try:"
  echo "sudo $0 \"$1\" $2"
  exit 1
fi

PUBLIC_URL="$1"

COUNT=`ls -1 *.osm.pbf 2>/dev/null | wc -l`
if [ $COUNT -gt "1" ]; then 
  echo "Only one .osm.pbf file is supported for now. There are ${COUNT} files:"
  ls -1 *osm.pbf
  exit 1
fi
if [ $COUNT -eq "1" ]; then 
  CUSTOM_EXTRACT=true
  AREA="`ls -1 *.osm.pbf | awk -F '.' '{print $1}'`"
  COUNTRIES=$2
  echo "✔️ Using custom extract ${AREA}.osm.pbf. This file will be moved to /home/${USER}/headway/"
  mv ${AREA}.osm.pbf /tmp/
else
  CUSTOM_EXTRACT=false
  AREA="$2"
  if [[ "$AREA" =~ ^(Aachen|Aarhus|Adelaide|Albuquerque|Alexandria|Amsterdam|Antwerpen|Arnhem|Auckland|Augsburg|Austin|Baghdad|Baku|Balaton|Bamberg|Bangkok|Barcelona|Basel|Beijing|Beirut|Berkeley|Berlin|Bern|Bielefeld|Birmingham|Bochum|Bogota|Bombay|Bonn|Bordeaux|Boulder|BrandenburgHavel|Braunschweig|Bremen|Bremerhaven|Brisbane|Bristol|Brno|Bruegge|Bruessel|Budapest|BuenosAires|Cairo|Calgary|Cambridge|CambridgeMa|Canberra|CapeTown|Chemnitz|Chicago|ClermontFerrand|Colmar|Copenhagen|Cork|Corsica|Corvallis|Cottbus|Cracow|CraterLake|Curitiba|Cusco|Dallas|Darmstadt|Davis|DenHaag|Denver|Dessau|Dortmund|Dresden|Dublin|Duesseldorf|Duisburg|Edinburgh|Eindhoven|Emden|Erfurt|Erlangen|Eugene|Flensburg|FortCollins|Frankfurt|FrankfurtOder|Freiburg|Gdansk|Genf|Gent|Gera|Glasgow|Gliwice|Goerlitz|Goeteborg|Goettingen|Graz|Groningen|Halifax|Halle|Hamburg|Hamm|Hannover|Heilbronn|Helsinki|Hertogenbosch|Huntsville|Innsbruck|Istanbul|Jena|Jerusalem|Johannesburg|Kaiserslautern|Karlsruhe|Kassel|Katowice|Kaunas|Kiel|Kiew|Koblenz|Koeln|Konstanz|LakeGarda|LaPaz|LaPlata|Lausanne|Leeds|Leipzig|Lima|Linz|Lisbon|Liverpool|Ljubljana|Lodz|London|Luebeck|Luxemburg|Lyon|Maastricht|Madison|Madrid|Magdeburg|Mainz|Malmoe|Manchester|Mannheim|Marseille|Melbourne|Memphis|MexicoCity|Miami|Minsk|Moenchengladbach|Montevideo|Montpellier|Montreal|Moscow|Muenchen|Muenster|NewDelhi|NewOrleans|NewYork|Nuernberg|Oldenburg|Oranienburg|Orlando|Oslo|Osnabrueck|Ostrava|Ottawa|Paderborn|Palma|PaloAlto|Paris|Perth|Philadelphia|PhnomPenh|Portland|PortlandME|Porto|PortoAlegre|Potsdam|Poznan|Prag|Providence|Regensburg|Riga|RiodeJaneiro|Rostock|Rotterdam|Ruegen|Saarbruecken|Sacramento|Saigon|Salzburg|SanFrancisco|SanJose|SanktPetersburg|SantaBarbara|SantaCruz|Santiago|Sarajewo|Schwerin|Seattle|Seoul|Sheffield|Singapore|Sofia|Stockholm|Stockton|Strassburg|Stuttgart|Sucre|Sydney|Szczecin|Tallinn|Tehran|Tilburg|Tokyo|Toronto|Toulouse|Trondheim|Tucson|Turin|UlanBator|Ulm|Usedom|Utrecht|Vancouver|Victoria|WarenMueritz|Warsaw|WashingtonDC|Waterloo|Wien|Wroclaw|Wuerzburg|Wuppertal|Zagreb|Zuerich)$ ]]; then 
    echo "✔️ Using predefined extract for: ${AREA}"
  else
    echo "💥 ${AREA} is not a valid available predefined extract and no .osm.pbf file is present on `pwd`. Run $0 to see the list of available predefined extracts or add your .osm.pbf. file"
    exit 1
  fi
fi
echo "✨ Upgrading the system..."
apt update && apt upgrade -y && apt dist-upgrade -y && apt autoremove -y

echo "✨ Creating user ${USER} with a random password..."
if [  ! -d "/home/${USER}" ]; then
  PASS=`< /dev/urandom tr -dc _A-Z-a-z-0-9 | head -c${3:-50};echo;`
  CRYPT=$(perl -e 'print crypt($ARGV[0], "password")' $PASS)
  useradd -m -p "$CRYPT" "${USER}"
  groupadd -f docker
  adduser ${USER} docker
else
  echo "${USER} already exists!"
fi

echo "✨ Installing Docker..."
if ! hash docker &> /dev/null
then
  apt install -y ca-certificates curl gnupg lsb-release
  curl -fsSL https://download.docker.com/linux/debian/gpg | gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
  echo \
  "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/debian \
  $(lsb_release -cs) stable" | tee /etc/apt/sources.list.d/docker.list > /dev/null 
  apt update && apt install -y docker-ce docker-ce-cli containerd.io
else
  echo "Docker seems to be already installed!"
fi

echo "✨ Installing Earthly..."
if ! hash earthly &> /dev/null
then
  apt install git docker-compose-plugin -y 
  /bin/sh -c 'wget https://github.com/earthly/earthly/releases/latest/download/earthly-linux-amd64 -O /usr/local/bin/earthly && chmod +x /usr/local/bin/earthly && /usr/local/bin/earthly bootstrap --with-autocomplete'
else
  echo "Earthly seems to be already installed!"
fi

echo "✨ Installing Headway..."
if [  ! -d "/home/${USER}/headway" ]; then
  cd /home/${USER}
  sudo -u ${USER} git clone https://github.com/headwaymaps/headway
  cd headway
  sudo -u ${USER} echo "HEADWAY_AREA=${AREA}
HEADWAY_PUBLIC_URL=${PUBLIC_URL}
HEADWAY_HTTP_PORT=${PORT}" > .env
else
  echo  "Headway seems to be already installed!"
  cd /home/${USER}/headway
fi

echo "✨ Processing ${AREA}..."
if [ "$CUSTOM_EXTRACT" = true ] ; then
  mv /tmp/${AREA}.osm.pbf .
  sudo -u ${USER} earthly -P +build --area=${AREA} --countries="${COUNTRIES}"
else
  sudo -u ${USER} earthly -P +build --area=${AREA}
fi

echo "✨ Running the containers..."
sudo -u ${USER} docker compose up -d

echo "✨ Adding start, stop and reload scripts..."
echo "#!/bin/bash
set -e
cd /home/${USER}/headway
docker compose up -d" > /usr/local/bin/run_headway.sh
chmod +x /usr/local/bin/run_headway.sh

echo "#!/bin/bash
set -e
cd /home/${USER}/headway
docker compose down" > /usr/local/bin/stop_headway.sh
chmod +x /usr/local/bin/stop_headway.sh

echo "#!/bin/bash
set -e
cd /home/${USER}/headway
git config pull.ff only
git pull" > /usr/local/bin/upgrade_and_recreate_headway.sh
if [ "$CUSTOM_EXTRACT" = true ] ; then
  echo "# Uncomment the next line and modify the URL to enable the automatic daily download of your fresh custom extract."
  echo "#wget -nv https://custom.site/customextract.pbf -O /home/${USER}/headway/mycustomextract.osm.pbf" >> /usr/local/bin/upgrade_and_recreate_headway.sh
  echo "earthly prune && earthly build -P +build --area=${AREA} --countries=${COUNTRIES}"  >> /usr/local/bin/upgrade_and_recreate_headway.sh
else
   echo "earthly prune && earthly build -P +build --area=${AREA}" >> /usr/local/bin/upgrade_and_recreate_headway.sh
fi
echo "docker-compose kill && docker-compose down --volumes && docker-compose up -d" >> /usr/local/bin/upgrade_and_recreate_headway.sh 
chmod +x /usr/local/bin/upgrade_and_recreate_headway.sh

echo "✨ Setting up the systemd service..."
echo "# Headway service
[Unit]
Description=Headway Maps
Documentation=https://github.com/headwaymaps/headway
After=network.target
StartLimitIntervalSec=0
StartLimitAction=reboot

[Service] 
Type=onshot
RemainAfterExit=true
User= ${USER}
ExecStart=/usr/local/bin/run_headway.sh
ExecStop=/usr/local/bin/stop_headway.sh
ExecReload=/usr/local/bin/upgrade_and_recreate_headway.sh

[Install] 
WantedBy= multi-user.target" > /etc/systemd/system/headway.service

systemctl enable headway

echo "✨ Setting up cron..."
RANDOM_HOUR=`date --date "1970-01-01 00:00:00 $(shuf -n1 -i0-21600) sec" '+%H'`
RANDOM_MINUTE=`date --date "1970-01-01 00:00:00 $(shuf -n1 -i0-21600) sec" '+%M'`
echo "Setting up cron to upgrade and recreate Headway with fresh data, at a random time between 00:00 and 6:00. Exactly at ${RANDOM_HOUR}:${RANDOM_MINUTE}"
echo "${RANDOM_MINUTE} ${RANDOM_HOUR}	* * *	$USER	/usr/local/bin/upgrade_and_recreate_headway.sh" >> /etc/crontab
service cron restart

echo "✨ Restarting the service..."
service headway restart

echo "✨ Waiting $WAIT_SECONDS seconds and checking that all is good..."
sleep $WAIT_SECONDS
if wget -nv ${PUBLIC_URL} -O /tmp/test ; then
  echo "All done! 🥳 Headway is accesible at ${PUBLIC_URL}"
else
  echo "Headway is installed! 🥳 But it is not accessible at ${PUBLIC_URL} ☝️ 🤔"
  GUESSED_IP=`ip a s | grep inet | grep -v inet6 | head -n2 | tail -n1 | awk '{print $2}' | awk -F '/' '{print $1}'`
  echo "⚠️ Check that your proxy is properly configured to redirect the web traffic to the port 8080 on this machine. Try http://${GUESSED_IP}:8080 and http://localhost:8080"
fi

if [ "$CUSTOM_EXTRACT" = true ] ; then
  echo " ✍️ Edit /usr/local/bin/upgrade_and_recreate_headway.sh if you want to enable the automatic daily download of your fresh custom pbf extract."
fi

exit 0
