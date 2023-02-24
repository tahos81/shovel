CREATE TABLE erc1155_balance(
  id INT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  contract_address VARCHAR(80) NOT NULL,
  token_id_low VARCHAR(80) NOT NULL,
  token_id_high VARCHAR(80) NOT NULL,
  balance_low VARCHAR(80) NOT NULL,
  balance_high VARCHAR(80) NOT NULL,
  last_update_block BIGINT DEFAULT 0
);
