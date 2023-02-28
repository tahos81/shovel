CREATE TABLE erc1155_balances(
  id INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  contract_address VARCHAR(80) NOT NULL,
  token_id_low VARCHAR(80) NOT NULL,
  token_id_high VARCHAR(80) NOT NULL,
  account VARCHAR(80) NOT NULL,
  balance_low VARCHAR(80) NOT NULL,
  balance_high VARCHAR(80) NOT NULL,
  last_updated_block BIGINT DEFAULT 0
);
