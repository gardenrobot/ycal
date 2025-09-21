#!/bin/bash

docker run -p 5232:5232 -v ./caldav-server/users:/etc/radicale/users -v ./caldav-server/config:/etc/radicale/config -d --name radicale radicale
sleep 2
curl -X MKCOL -H 'Authorization:Basic dXNlcjpwYXNz' \
    --data '<?xml version="1.0" encoding="UTF-8" ?><mkcol xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav" xmlns:CR="urn:ietf:params:xml:ns:carddav" xmlns:CS="http://calendarserver.org/ns/"><set><prop><resourcetype><collection /><C:calendar /></resourcetype><C:supported-calendar-component-set><C:comp name="VEVENT" /></C:supported-calendar-component-set></prop></set></mkcol>' \
    'http://localhost:5232/user/cal/'
docker logs -f radicale

docker rm --force radicale