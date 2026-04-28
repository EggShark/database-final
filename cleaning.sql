-- Thing used to do extra cleaning

-- set search path to area needs to be. When group perms are set up change to group
SET search_path TO group120800, public;

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

UPDATE alpr
  SET manufacturer = 'Ekin Box Spotter'
WHERE manufacturer ILIKE 'ekin%';

UPDATE alpr SET
  manufacturer = 'Axis Communications'
WHERE manufacturer ILIKE '%axis%';

UPDATE alpr SET 
  manufacturer = 'Axon Enterprise'
WHERE manufacturer ILIKE '%axon%';

-- I LOVE TYPOS AND INCONSISTENT DATA IN MY DATA SET <3
UPDATE alpr SET 
  manufacturer = 'Cyber Secure'
WHERE manufacturer ILIKE '%yber secur%';


UPDATE alpr SET
  manufacturer = 'Bosh Security Systems'
WHERE manufacturer ILIKE 'bosch%';

-- oh yeah single typo in data set!!
UPDATE alpr SET 
  manufacturer = 'Genetec'
WHERE manufacturer = 'Genetech';

UPDATE alpr SET
  manufacturer = 'ICamera'
WHERE manufacturer = 'I';

UPDATE alpr SET
  manufacturer = 'Kapsch'
WHERE manufacturer ILIKE 'Kapsch%';


UPDATE alpr SET 
  manufacturer = REPLACE(manufacturer, '?', '');

-- Incosistent with Inc and LLC just doing all INC as that has more entries
-- Too lazy to find acually incorporation status online
UPDATE alpr SET
  manufacturer = 'Leonardo'
WHERE manufacturer ILIKE 'Leonardo %';

UPDATE alpr SET
  manufacturer = 'LiveView Technologies'
WHERE manufacturer ILIKE 'liveview%' OR manufacturer ILIKE 'live view %' OR manufacturer ILIKE 'lifeview%'; 

UPDATE alpr SET 
  manufacturer = 'Neology, Inc.'
WHERE manufacturer ILIKE 'neology%';

UPDATE alpr SET 
  manufacturer = 'Insight LPR'
WHERE manufacturer ILIKE 'Insight%';

-- Another top ten inconsistency and inc moment
UPDATE alpr SET
  manufacturer = 'Verkada'
WHERE manufacturer ILIKE 'verkada';

UPDATE alpr SET 
  manufacturer = 'Mobotix'
WHERE manufacturer = 'Mobitix';

UPDATE alpr SET
  manufacturer = 'Rekor'
WHERE manufacturer ILIKE 'Rekor%' OR manufacturer = 'Rektor';

UPDATE alpr SET 
  manufacturer = 'PlateSmart'
WHERE manufacturer ILIKE 'platesmart%';

UPDATE alpr SET
  manufacturer = 'Verkada'
WHERE manufacturer = 'Verkada Inc.';

UPDATE alpr SET
  manufacturer = 'Uniview'
WHERE manufacturer ILIKE 'Uniview%' OR manufacturer = 'Unv';

UPDATE alpr SET
  manufacturer = 'LVT'
WHERE manufacturer = 'LVT Mobile Tower';

UPDATE alpr SET
  manufacturer = 'generic'
WHERE manufacturer ILIKE 'unkn%' OR manufacturer = 'Unkwn' OR manufacturer = 'Unkown' OR manufacturer = 'other';

-- catches the many typos and compound things in Flock Saftey
UPDATE alpr SET
  manufacturer = 'Flock Saftey'
WHERE manufacturer ILIKE '%floc%' or manufacturer ILIKE 'flow safety';

UPDATE alpr SET 
  manufacturer = 'Motorola Solutions'
WHERE manufacturer ILIKE '%motorola%' OR manufacturer ILIKE '%motorolla%' or manufacturer ILIKE '%mortorola%';

SELECT COUNT(*), manufacturer FROM alpr GROUP BY manufacturer;
