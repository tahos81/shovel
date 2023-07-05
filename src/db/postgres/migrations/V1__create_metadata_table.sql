CREATE TYPE t_contract_type AS ENUM ('ERC721', 'ERC1155');

CREATE TABLE contract_metadata(
  "id" INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  "contract_address" VARCHAR(80) NOT NULL,
  "owner" VARCHAR(80),
  "contract_type" t_contract_type NOT NULL,
  "name" TEXT,
  "symbol" TEXT,
  "base_uri" TEXT,
  "last_updated_block" BIGINT DEFAULT 0
);

CREATE TABLE token_metadata(
  "id" INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  "contract_address" VARCHAR(80) NOT NULL,
  "contract_type" t_contract_type NOT NULL,
  "token_id_low" VARCHAR(80) NOT NULL,
  "token_id_high" VARCHAR(80) NOT NULL,
  -- Metadata
  "image" TEXT,
  "image_data" TEXT,
  "external_url" TEXT,
  "description" TEXT,
  "name" TEXT,
  "background_color" TEXT,
  "animation_url" TEXT,
  "youtube_url" TEXT
);

CREATE TABLE token_metadata_attributes(
  "id" INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  "token_metadata_id" INT NOT NULL,
  "value" TEXT,
  "display_type" TEXT,
  "trait_type" TEXT,

  -- token_metadata_attributes[token_metadata_id] -> token_metadata[id]
  CONSTRAINT "fk_token_metadata"
    FOREIGN KEY("token_metadata_id")
    REFERENCES token_metadata("id")
    ON DELETE CASCADE
);
