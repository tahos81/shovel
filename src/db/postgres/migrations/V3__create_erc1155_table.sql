CREATE TABLE erc1155_token(
  "id" INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  "contract_id" INT NOT NULL,
  "contract_address" VARCHAR(80) NOT NULL,
  "token_id_low" VARCHAR(80) NOT NULL,
  "token_id_high" VARCHAR(80) NOT NULL,
  "token_uri" TEXT,
  "last_updated_block" BIGINT DEFAULT 0,

  -- erc1155_token[contract_id] -> contract[id]
  CONSTRAINT "fk_contract"
    FOREIGN KEY("contract_id")
    REFERENCES contract_metadata("id")
    ON DELETE CASCADE
);

CREATE TABLE erc1155_balances(
  "id" INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  "erc1155_id" INT NOT NULL,
  "account" VARCHAR(80) NOT NULL,
  "balance_low" VARCHAR(80) NOT NULL,
  "balance_high" VARCHAR(80) NOT NULL,
  "last_updated_block" BIGINT DEFAULT 0,

  -- erc1155_balances[erc1155_id] -> erc1155_token[id]
  CONSTRAINT "fk_erc1155"
    FOREIGN KEY("erc1155_id")
    REFERENCES erc1155_token("id")
    ON DELETE CASCADE
);
