#!/bin/bash

exec=/app/ycal

start_date=$(date +%F)
days_before=7
days_after=30

end_date=$(date --date "$start_date + $days_after days" +%F)
current_date=$(date --date "$start_date - $days_before days" +%F)
while [[ "$current_date" != "$end_date" ]]; do
    $exec process $current_date
    current_date=$(date --date "$current_date +1 days" +%F)
done

#date --date "2025-01-01 +0 day" +%F