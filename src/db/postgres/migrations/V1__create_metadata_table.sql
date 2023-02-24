CREATE TYPE t_contract_type AS ENUM ('ERC721', 'ERC1155');

CREATE TABLE contract_metadata(
  id INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  contract_address VARCHAR(80) NOT NULL,
  contract_type t_contract_type NOT NULL,
  name TEXT,
  symbol TEXT,
  last_updated INTEGER
);

CREATE TABLE token_metadata(
  id INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  contract_address VARCHAR(80) NOT NULL,
  contract_type t_contract_type NOT NULL,
  token_id_low VARCHAR(80) NOT NULL,
  token_id_high VARCHAR(80) NOT NULL,
  metadata TEXT NOT NULL
);
