meta:
  id: alpha__block_header__unsigned
  endian: be
types:
  alpha__block_header__alpha__unsigned_contents:
    seq:
    - id: payload_hash
      size: 32
    - id: payload_round
      type: s4
    - id: proof_of_work_nonce
      size: 8
    - id: seed_nonce_hash_tag
      type: u1
      enum: bool
    - id: seed_nonce_hash
      size: 32
      if: (seed_nonce_hash_tag == bool::true)
    - id: per_block_votes
      type: u1
      enum: alpha__per_block_votes_tag
  block_header__shell:
    doc: ! >-
      Shell header: Block header's shell-related content. It contains information
      such as the block level, its predecessor and timestamp.
    seq:
    - id: level
      type: s4
    - id: proto
      type: u1
    - id: predecessor
      size: 32
    - id: timestamp
      type: s8
      doc: ! 'A timestamp as seen by the protocol: second-level precision, epoch based.'
    - id: validation_pass
      type: u1
    - id: operations_hash
      size: 32
    - id: fitness
      type: fitness
    - id: context
      size: 32
  fitness:
    doc: ! >-
      Block fitness: The fitness, or score, of a block, that allow the Tezos to decide
      which chain is the best. A fitness value is a list of byte sequences. They are
      compared as follows: shortest lists are smaller; lists of the same length are
      compared according to the lexicographical order.
    seq:
    - id: len_fitness
      type: s4
    - id: fitness
      type: fitness_entries
      size: len_fitness
      repeat: eos
  fitness_entries:
    seq:
    - id: fitness__elem
      type: fitness__elem
  fitness__elem:
    seq:
    - id: len_fitness__elem
      type: s4
    - id: fitness__elem
      size: len_fitness__elem
enums:
  alpha__per_block_votes_tag:
    0: case__0
    4: case__4
    8: case__8
    1: case__1
    5: case__5
    9: case__9
    2: case__2
    6: case__6
    10: case__10
  bool:
    0: false
    255: true
seq:
- id: block_header__shell
  type: block_header__shell
- id: alpha__block_header__alpha__unsigned_contents
  type: alpha__block_header__alpha__unsigned_contents