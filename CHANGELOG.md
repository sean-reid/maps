# Changelog

## 0.1.0 (2026-11-15)


### Features

* add FAOSTAT crops, livestock, land, forestry and inputs ([#18](https://github.com/sean-reid/maps/issues/18)) ([9bbddd1](https://github.com/sean-reid/maps/commit/9bbddd11fb57acf15e692b0976cb17e1ebd06ccf))
* add the cartogram solver crate ([#3](https://github.com/sean-reid/maps/issues/3)) ([a4dad50](https://github.com/sean-reid/maps/commit/a4dad50579b70ddcea65c9934f07323d2ab6f84e))
* add the country topology and its grid layout ([#4](https://github.com/sean-reid/maps/issues/4)) ([a112e3f](https://github.com/sean-reid/maps/commit/a112e3fef28bbd4224f09fd1f88cfc4e80795b24))
* add the shell page and health endpoint ([754a5f8](https://github.com/sean-reid/maps/commit/754a5f86b2db274ce420a141a06ac6c38ea50922))
* add the World Bank ingest and the series format ([#6](https://github.com/sean-reid/maps/issues/6)) ([7b43d74](https://github.com/sean-reid/maps/commit/7b43d749ab9dab0b27890d3f5c159471a9446eff))
* animate the map through years ([#9](https://github.com/sean-reid/maps/issues/9)) ([c4072d8](https://github.com/sean-reid/maps/commit/c4072d84eb94cddf1075aaccc16568a956b681f0))
* draw the map with labels, zoom, pan and a tap readout ([#5](https://github.com/sean-reid/maps/issues/5)) ([83e5b70](https://github.com/sean-reid/maps/commit/83e5b70d1abfbed41247421b21d7842d8c6a8c1a))
* open on the population cartogram ([#8](https://github.com/sean-reid/maps/issues/8)) ([f935fde](https://github.com/sean-reid/maps/commit/f935fdecd3e00629ba416b90ad882dab1d990de7))
* search the series ([#10](https://github.com/sean-reid/maps/issues/10)) ([ca858f5](https://github.com/sean-reid/maps/commit/ca858f58fea40977043f2cb45245ced6e76e7432))
* serve series from R2 and refresh them monthly ([#7](https://github.com/sean-reid/maps/issues/7)) ([4b4153c](https://github.com/sean-reid/maps/commit/4b4153c199cf8097b6268d326e37a9904be5f70c))
* show historical entities as one country ([#19](https://github.com/sean-reid/maps/issues/19)) ([41e9c76](https://github.com/sean-reid/maps/commit/41e9c767611b4d8e1c2cdacc474011fa0fdb02d4))
* turn World Bank rates into totals where a denominator makes one ([#12](https://github.com/sean-reid/maps/issues/12)) ([ba394cc](https://github.com/sean-reid/maps/commit/ba394ccb8a2e89504d94805715534e53ea5de8c9))
* untangle outlines that cross themselves after a solve ([#20](https://github.com/sean-reid/maps/issues/20)) ([a7bbac3](https://github.com/sean-reid/maps/commit/a7bbac3376fca32bb355c22d99b7c54b64a20f30))


### Bug Fixes

* frame the solved map, not the plain one ([#15](https://github.com/sean-reid/maps/issues/15)) ([46dab18](https://github.com/sean-reid/maps/commit/46dab1896757712dd03d6cebaf4fe19b7df7e43b))
* let the Cloudflare Web Analytics beacon load ([#16](https://github.com/sean-reid/maps/issues/16)) ([54d46dd](https://github.com/sean-reid/maps/commit/54d46dde32f985f3ababfd92e0fa45197a96b328))
* offer only the years in which a series covers enough countries ([#17](https://github.com/sean-reid/maps/issues/17)) ([0bf5be0](https://github.com/sean-reid/maps/commit/0bf5be0ac4443e49db86633993c20541d93fba61))
* show the first map already in shape ([#13](https://github.com/sean-reid/maps/issues/13)) ([8c85b3a](https://github.com/sean-reid/maps/commit/8c85b3ad3a7da11949412cbb5835724f0084bd2b))
* solve every map from the undeformed one ([#14](https://github.com/sean-reid/maps/issues/14)) ([51a7a85](https://github.com/sean-reid/maps/commit/51a7a851b20a40a634d0754fd835e867b8edf9dd))
* stop sub-cell countries from stalling the solver ([#11](https://github.com/sean-reid/maps/issues/11)) ([4fe02fe](https://github.com/sean-reid/maps/commit/4fe02fe4d7ccdeea42123ebbb19688b2ae6a2fb2))
