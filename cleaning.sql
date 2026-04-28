-- Thing used to do extra cleaning

-- set search path to area needs to be. When group perms are set up change to group
SET search_path TO charlotte_crabtree, public;

BEGIN;
-- combine these two as there is no real distinction also mergies this into neighborhood_exit
UPDATE alpr SET
  surviellance_zone = 'residential'
WHERE surviellance_zone ILIKE '%neighborhood%' OR surviellance_zone ILIKE '%town%';

-- just merging all public transport monitoring into 1
UPDATE alpr SET
  surviellance_zone = 'public_transport'
WHERE surviellance_zone ILIKE '%bus_stop%' OR surviellance_zone ILIKE '%station%';

-- multiple things like parking_lot and parking_entrance so just make them all parkinging
UPDATE alpr SET 
  surviellance_zone = 'parking'
WHERE surviellance_zone ILIKE '%parking%';

-- again these are all commercial application so just merging them together
UPDATE alpr SET
  surviellance_zone = 'commercial'
WHERE surviellance_zone ILIKE '%shop%' OR surviellance_zone ILIKE '%mall%' OR surviellance_zone ILIKE '%building%';

-- same as above
UPDATE alpr SET
  surviellance_zone = 'entrance'
WHERE surviellance_zone ILIKE '%entrance%' OR surviellance_zone ILIKE '%exit%' OR surviellance_zone ILIKE '%gate%';

-- Grr ualbany
UPDATE alpr SET
  surviellance_zone = 'school'
WHERE surviellance_zone = 'ualbany';

-- Don't wanna hit parking with this so no wildcards
-- also gets outdoor_anti_dumping
UPDATE alpr SET
  surviellance_zone = 'outdoor'
WHERE surviellance_zone ILIKE 'park' OR surviellance_zone ILIKE '%outdoor%';

-- yeah we preserving;
UPDATE alpr SET
  surviellance_zone = 'public'
WHERE surviellance_zone ILIKE '%public%' AND NOT surviellance_zone ILIKE 'public_transport';

-- corrects common typos and also sets everything to traffic where its multiple
-- i.e traffic,street
-- also merges all categories that are traffic monitoring into roads
-- ky-207 is a highway grr there was a highway tag already the geo locational data tells us its on ky-207
UPDATE alpr SET 
  surviellance_zone = 'traffic'
WHERE surviellance_zone ILIKE '%traf%' OR surviellance_zone ILIKE '%street%' OR surviellance_zone ILIKE '%road%' OR surviellance_zone ILIKE '%intersection%' OR surviellance_zone ILIKE '%highway%' OR surviellance_zone ILIKE '%ky-207%';

-- Setting this to NULL as no meaning can be derived from this
UPDATE alpr SET
  surviellance_zone = NULL
WHERE surviellance_zone = 'area';

-- check results before commit
SELECT COUNT(*), surviellance_zone FROM alpr GROUP BY surviellance_zone;
