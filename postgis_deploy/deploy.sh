
files=(*.pgsql.tgz)

tar xvf ${files[0]}

psql -h postgis -U postgis_user -d postgis_db -a -f ./load.sql