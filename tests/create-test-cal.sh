#!/bin/bash

BASE_DIR="$(dirname -- "${BASH_SOURCE[0]}")"
URL='http://localhost:5232/user/cal/'

function start() {
    docker rm --force radicale
    docker build -t radicale "$BASE_DIR/caldav-server"
    docker run -p 5232:5232 \
        -v "$BASE_DIR/caldav-server/users:/etc/radicale/users" \
        -v "$BASE_DIR/caldav-server/config:/etc/radicale/config" \
        -d --name radicale radicale
    sleep 5
}

function remove() {
  docker rm --force radicale
}

function create_cal() {
    read -r -d '' payload <<- EOF
    <?xml version="1.0" encoding="UTF-8" ?>
    <mkcol xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav" xmlns:CR="urn:ietf:params:xml:ns:carddav" xmlns:CS="http://calendarserver.org/ns/">
        <set>
            <prop>
                <resourcetype><collection /><C:calendar /></resourcetype>
                <C:supported-calendar-component-set><C:comp name="VEVENT" /></C:supported-calendar-component-set>
            </prop>
        </set>
    </mkcol>
EOF
    curl -X MKCOL -H 'Authorization:Basic dXNlcjpwYXNz' \
        --data "$payload" \
        "$URL"
}

function watch() {
    docker logs -f radicale
    #docker rm --force radicale
}

function list_by_date {
    read -r -d '' payload <<- EOF
    <?xml version="1.0" encoding="utf-8" ?>
          <C:calendar-query xmlns:D="DAV:"
               xmlns:C="urn:ietf:params:xml:ns:caldav">
           <D:prop>
             <D:getetag/>
             <C:calendar-data>
               <C:comp name="VCALENDAR">
                 <C:prop name="VERSION"/>
                 <C:comp name="VEVENT">
                   <C:prop name="SUMMARY"/>
                   <C:prop name="UID"/>
                   <C:prop name="DTSTART"/>
                   <C:prop name="DTEND"/>
                   <C:prop name="DURATION"/>
                   <C:prop name="RRULE"/>
                   <C:prop name="RDATE"/>
                   <C:prop name="EXRULE"/>
                   <C:prop name="EXDATE"/>
                   <C:prop name="RECURRENCE-ID"/>
                 </C:comp>
                 <C:comp name="VTIMEZONE"/>
               </C:comp>
             </C:calendar-data>
           </D:prop>
           <C:filter>
             <C:comp-filter name="VCALENDAR">
               <C:comp-filter name="VEVENT">
                 <C:time-range start="20250101T000000Z"
                               end="20250102T000000Z"/>
               </C:comp-filter>
             </C:comp-filter>
           </C:filter>
          </C:calendar-query>
EOF
    curl -X GET \
        -H 'Authorization:Basic dXNlcjpwYXNz' \
        -H "Content-Type: text/xml" \
        -H "Depth: 1" \
        --data "$payload" \
        "http://127.0.0.1:5232/user/cal/"
}

#remove
#start
create_cal
#watch
#list_by_date