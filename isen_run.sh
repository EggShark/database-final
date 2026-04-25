#! /bin/bash

# build it
cargo b

# make sure you have isen_jumpbox in your ~/.ssh/config
# uplaods geojso (maybe make skipable)
scp ALPRs.geojson isen_jumbox:database-final/ALPRs.geojson
scp target/debug/database-final isen_jumpbox:database-final/database_uploader

ssh isen_jumpbox 'cd database-final; ./database_uploader'
