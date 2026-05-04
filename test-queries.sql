set search_path TO group120800, public;

-- -105.220660, 39.747135 long(lat) coords of roundabout on 19th street
-- query to get the closest camera to you units in meters?

-- -105.222559 39.746179 closer to camera
SELECT node_id, manufacturer, ST_AsText(position) as pos,
  position, operator,
  ST_Distance(
    ST_Transform(ST_SetSRID(ST_MakePoint(-105.222559 , 39.746179),4326), 3857),
    ST_Transform(position::geometry, 3857)
  ) * cosd(39.746179) as distance,
  -- ooogy moogy
  DEGREES(ST_Azimuth(position::geometry, ST_SetSRID(ST_MakePoint(-105.222559, 39.746179), 4326))) as angle
  FROM alpr ORDER BY 
    ST_Distance(
      ST_SetSRID(ST_MakePoint(-105.220660, 39.746179),4326),
      position
    )
    LIMIT 1;

SELECT * FROM alpr WHERE node_id IN (11232228563, 9232283, 891419347);
