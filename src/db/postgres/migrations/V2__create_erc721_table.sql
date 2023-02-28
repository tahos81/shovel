CREATE TABLE erc721_data (
  id INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  contract_address VARCHAR(80) NOT NULL,
  token_id_low VARCHAR(80) NOT NULL,
  token_id_high VARCHAR(80) NOT NULL,
  latest_owner VARCHAR(80),
  token_uri TEXT,
  last_updated_block BIGINT DEFAULT 0
);

CREATE TABLE erc721_owners(
  id INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  erc721_id INT NOT NULL,
  owner VARCHAR(80) NOT NULL,
  block BIGINT NOT NULL,

  -- Define foreign key to erc721_data.id
  CONSTRAINT fk_erc721
    FOREIGN KEY(erc721_id)
    REFERENCES erc721_data(id)
    ON DELETE CASCADE
);
