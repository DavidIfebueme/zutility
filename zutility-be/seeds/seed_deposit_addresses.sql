INSERT INTO deposit_addresses (address, address_type, used)
VALUES
  ('tmQ1Y8xQx5G4h5w6nJ4D31oQmRVVbYkA4W', 'transparent', false),
  ('tmQ5xhPpQx3VxvMBM3Q5F2nq6qWRwR4XW7', 'transparent', false),
  ('tmV9LkX8V2w4jB8Dk8kZ8rj8p2Pz5g6Qm1', 'transparent', false),
  ('ztestsapling1q3f4v8k6e4q7s9x2a5w6d8j9m3k2t7y8u6i5o4p3l2k1j0h9g8f7d6', 'shielded', false),
  ('ztestsapling1q8m4c6v2b7n5a1s9d3f6g2h8j4k7l1z5x9c3v7b2n6m4q8w1e5r', 'shielded', false),
  ('ztestsapling1q2w4e6r8t1y3u5i7o9p2a4s6d8f1g3h5j7k9l2z4x6c8v1b3n5m', 'shielded', false)
ON CONFLICT (address) DO NOTHING;
