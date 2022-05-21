DROP INDEX IF EXISTS old_idx_nodes_geom;
DROP INDEX IF EXISTS old_idx_way_nodes_node_id;
DROP INDEX IF EXISTS old_idx_ways_bbox;
DROP INDEX IF EXISTS old_idx_ways_linestring;
DROP INDEX IF EXISTS old_idx_node_tags_node_id;
DROP INDEX IF EXISTS old_idx_way_tags_way_id;
DROP INDEX IF EXISTS old_idx_relation_tags_relation_id;

DROP TABLE IF EXISTS "old_nodes";
DROP TABLE IF EXISTS "old_relation_members";
DROP TABLE IF EXISTS "old_relations";
DROP TABLE IF EXISTS "old_users";
DROP TABLE IF EXISTS "old_way_nodes";
DROP TABLE IF EXISTS "old_ways";

DROP INDEX IF EXISTS new_idx_nodes_geom;
DROP INDEX IF EXISTS new_idx_way_nodes_node_id;
DROP INDEX IF EXISTS new_idx_ways_bbox;
DROP INDEX IF EXISTS new_idx_ways_linestring;
DROP INDEX IF EXISTS new_idx_node_tags_node_id;
DROP INDEX IF EXISTS new_idx_way_tags_way_id;
DROP INDEX IF EXISTS new_idx_relation_tags_relation_id;

DROP TABLE IF EXISTS "new_nodes";
DROP TABLE IF EXISTS "new_relation_members";
DROP TABLE IF EXISTS "new_relations";
DROP TABLE IF EXISTS "new_users";
DROP TABLE IF EXISTS "new_way_nodes";
DROP TABLE IF EXISTS "new_ways";

CREATE TABLE "new_nodes" (LIKE "nodes");
CREATE TABLE "new_relation_members" (LIKE "relation_members");
CREATE TABLE "new_relations" (LIKE "relations");
CREATE TABLE "new_users" (LIKE "users");
CREATE TABLE "new_way_nodes" (LIKE "way_nodes");
CREATE TABLE "new_ways" (LIKE "ways");

\copy new_nodes FROM 'nodes.txt'
\copy new_relation_members FROM 'relation_members.txt'
\copy new_relations FROM 'relations.txt'
\copy new_users FROM 'users.txt'
\copy new_way_nodes FROM 'way_nodes.txt'
\copy new_ways FROM 'ways.txt'
\copy new_node_tags FROM 'node_tags.txt'
\copy new_way_tags FROM 'way_tags.txt'
\copy new_relation_tags FROM 'relation_tags.txt'

ALTER TABLE ONLY new_nodes ADD CONSTRAINT pk_nodes PRIMARY KEY (id);
ALTER TABLE ONLY new_ways ADD CONSTRAINT pk_ways PRIMARY KEY (id);
ALTER TABLE ONLY new_way_nodes ADD CONSTRAINT pk_way_nodes PRIMARY KEY (way_id, sequence_id);
ALTER TABLE ONLY new_relations ADD CONSTRAINT pk_relations PRIMARY KEY (id);
ALTER TABLE ONLY new_relation_members ADD CONSTRAINT pk_relation_members PRIMARY KEY (relation_id, sequence_id);

CREATE INDEX new_idx_nodes_geom ON new_nodes USING gist (geom);
CREATE INDEX new_idx_way_nodes_node_id ON new_way_nodes USING btree (node_id);
CREATE INDEX new_idx_ways_bbox ON new_ways USING gist (bbox);
CREATE INDEX new_idx_ways_linestring ON new_ways USING gist (linestring);
CREATE INDEX new_idx_node_tags_node_id ON new_node_tags USING btree (node_id);
CREATE INDEX new_idx_way_tags_way_id ON new_way_tags USING btree (way_id);
CREATE INDEX new_idx_relation_tags_relation_id ON new_relation_tags USING btree (relation_id);

BEGIN;

ALTER TABLE "nodes" RENAME TO "old_nodes";
ALTER TABLE "relation_members" RENAME TO "old_relation_members";
ALTER TABLE "relations" RENAME TO "old_relations";
ALTER TABLE "users" RENAME TO "old_users";
ALTER TABLE "way_nodes" RENAME TO "old_way_nodes";
ALTER TABLE "ways" RENAME TO "old_ways";

ALTER TABLE "new_nodes" RENAME TO "nodes";
ALTER TABLE "new_relation_members" RENAME TO "relation_members";
ALTER TABLE "new_relations" RENAME TO "relations";
ALTER TABLE "new_users" RENAME TO "users";
ALTER TABLE "new_way_nodes" RENAME TO "way_nodes";
ALTER TABLE "new_ways" RENAME TO "ways";

ALTER INDEX idx_nodes_geom RENAME TO old_idx_nodes_geom;
ALTER INDEX idx_way_nodes_node_id RENAME TO old_idx_way_nodes_node_id;
ALTER INDEX idx_ways_bbox RENAME TO old_idx_ways_bbox;
ALTER INDEX idx_ways_linestring RENAME TO old_idx_ways_linestring;
ALTER INDEX idx_node_tags_node_id RENAME TO old_idx_node_tags_node_id;
ALTER INDEX idx_way_tags_way_id RENAME TO old_idx_way_tags_way_id;
ALTER INDEX idx_relation_tags_relation_id RENAME TO old_idx_relation_tags_relation_id;

ALTER INDEX new_idx_nodes_geom RENAME TO idx_nodes_geom;
ALTER INDEX new_idx_way_nodes_node_id RENAME TO idx_way_nodes_node_id;
ALTER INDEX new_idx_ways_bbox RENAME TO idx_ways_bbox;
ALTER INDEX new_idx_ways_linestring RENAME TO idx_ways_linestring;
ALTER INDEX new_idx_node_tags_node_id RENAME TO idx_node_tags_node_id;
ALTER INDEX new_idx_way_tags_way_id RENAME TO idx_way_tags_way_id;
ALTER INDEX new_idx_relation_tags_relation_id RENAME TO idx_relation_tags_relation_id;

COMMIT;

DROP INDEX IF EXISTS old_idx_nodes_geom;
DROP INDEX IF EXISTS old_idx_way_nodes_node_id;
DROP INDEX IF EXISTS old_idx_ways_bbox;
DROP INDEX IF EXISTS old_idx_ways_linestring;
DROP INDEX IF EXISTS old_idx_node_tags_node_id;
DROP INDEX IF EXISTS old_idx_way_tags_way_id;
DROP INDEX IF EXISTS old_idx_relation_tags_relation_id;

DROP TABLE IF EXISTS "old_nodes";
DROP TABLE IF EXISTS "old_relation_members";
DROP TABLE IF EXISTS "old_relations";
DROP TABLE IF EXISTS "old_users";
DROP TABLE IF EXISTS "old_way_nodes";
DROP TABLE IF EXISTS "old_ways";

VACUUM ANALYZE;