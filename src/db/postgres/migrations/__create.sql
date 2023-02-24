CREATE DATABASE shovel;

-- Create shovel_user
CREATE ROLE shovel_user WITH LOGIN ENCRYPTED PASSWORD 'shovel_1234';

-- Gives access for all tables in namespace 'public'
-- GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO shovel_user;

-- Gives admin accesses for database
GRANT ALL PRIVILEGES ON DATABASE shovel TO shovel_user;
