ALTER TABLE deposit_addresses
  DROP CONSTRAINT fk_deposit_addresses_order,
  ADD CONSTRAINT fk_deposit_addresses_order
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE SET NULL
    DEFERRABLE INITIALLY DEFERRED;
