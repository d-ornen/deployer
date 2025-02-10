#!/usr/bin/env python

import tomllib

with open('Cargo.toml', 'rb') as f:
  data = tomllib.load(f)
  print(data['package']['version'])
