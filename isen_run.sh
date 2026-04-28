#! /bin/bash

# build it
cargo b --bin database-uploader

# make sure you have isen_jumpbox in your ~/.ssh/config
# uplaods geojso (maybe make skipable)
scp ALPRs.geojson isen_jumpbox:database-final/ALPRs.geojson
scp target/debug/database-uploader isen_jumpbox:database-final/database_uploader

ssh isen_jumpbox 'cd database-final; ./database_uploader'
