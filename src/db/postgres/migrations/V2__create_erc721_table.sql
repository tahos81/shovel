CREATE TABLE erc721_token(
  "id" INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  "contract_id" INT NOT NULL,
  "contract_address" VARCHAR(80) NOT NULL,
  "token_id_low" VARCHAR(80) NOT NULL,
  "token_id_high" VARCHAR(80) NOT NULL,
  "latest_owner" VARCHAR(80),
  "token_uri" TEXT,
  "last_updated_block" BIGINT DEFAULT 0,
  
  -- erc721_token[contract_id] -> contract[id]
  CONSTRAINT "fk_contract"
    FOREIGN KEY("contract_id")
    REFERENCES contract_metadata("id")
    ON DELETE CASCADE
);

CREATE TABLE erc721_owners(
  "id" INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  "erc721_id" INT NOT NULL,
  "owner" VARCHAR(80) NOT NULL,
  "block" BIGINT NOT NULL,

  -- erc721_owners[erc721_id] -> erc721_token[id]
  CONSTRAINT "fk_erc721"
    FOREIGN KEY("erc721_id")
    REFERENCES erc721_token("id")
    ON DELETE CASCADE
);
