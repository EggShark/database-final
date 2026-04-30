# CSCI 403 Database Management Final project
Dataset: All ALRPs in the united states (Including Alaska and Florida)

## Set up
Make sure to have a file titled secret in the root directory of the project that looks like
```
username
password
```
otherwise it will not compile and you cannot log into your database
### Mines student who is on campus
You can either run `cargo r --bin database-uploader` on your computer to updload all the data into ADA or SCP the files into isengard like:

`scp ALPRs.geojson user_name@isengard.mines.edu:ALPRs.geojson`

`scp target/debug/database-uploader user_name@isengard.mines.edu:database-uploader`

then run it like:

`ssh user_name@isengard.mines.edu './database-uploader -s scheema'`

Where the scheema you enter is the desired output scheema.

just make sure `ALPRs.geojson` is in the same directory as the executable.
### Mines student off campus
In your `.ssh` file set up and entry called `isen_jumpbox` that connects to the isengard jumpox on campus then just run `.isen_run.sh` on your local machine

Do note that as of right now `.isen_run.sh` has no way to configure what scheema to updload to and will just upload to your personal scheema 
### Non mines student
Set up a local/personal postgress database with postGIS and edit then provide database uploader with a valid url to your postgress database like

`cargo r -- -u mydatabase.coolsite.swag`

Data taken from open street map and their contributers
