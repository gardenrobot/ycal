#!/bin/bash
# Take 2 dates and run the create subcommand on each date in between.

d=2024-07-01
while [ "$d" != 2024-09-01 ]; do 
  echo $d
  cargo run create $d

  d=$(date -I -d "$d + 1 day")
done
