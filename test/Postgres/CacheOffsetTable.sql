CREATE TABLE IF NOT EXISTS public."CacheOffsetTable"
(
    date date NOT NULL,
    collection  text NOT NULL,
    "offset" bigint NOT NULL DEFAULT 0,
    CONSTRAINT "CacheOffsetTable_pkey" PRIMARY KEY (date, collection)
)