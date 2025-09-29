# Changelog

## [0.3.0] - 2025-09-29

### BREAKING CHANGES

- update `swc_core` to `43.0`

### Miscellaneous Tasks

- **\[breaking\]** update `swc_core` to `43.0`

## [0.2.0] - 2025-01-21

### BREAKING CHANGES

- update `swc_core` to `10.6`

### Miscellaneous Tasks

- **\[breaking\]** update `swc_core` to `10.6`

## [0.1.4] - 2024-11-06

### Bug Fixes

- fix the list of files to be packed

## [0.1.3] - 2024-11-06

### Bug Fixes

- **(graphql-minify)** potentially fix stack overflow during parsing of very long strings

### Miscellaneous Tasks

- **(license)** use dual Unlicense/MIT license

### Performance

- **(graphql-minify)** move comment from `skip` to a separate token

### Testing

- add fuzzing and test randomly generated queries and mutations

## [0.1.2] - 2024-10-26

### Bug Fixes

- include `graphql-minify` license in pack

### Features

- print the parsing error span relative to the file

### Performance

- re-use single allocator for `graphql_minify::minify`

## [0.1.1] - 2024-10-22

### Bug Fixes

- fix `license` to `Unlicense`

## [0.1.0] - 2024-10-22

### Features

- minimize string and template literals
- report error span
- **(graphql-minify)** do not print space before ellipsis

### Miscellaneous Tasks

- **(bench)** add minify benchmarks

### Performance

- **(graphql-minify)** do not re-split lines in `print_block_string`
- **(graphql-minify)** use `bumpalo` for allocations
- **(graphql-minify)** do not store pointers in `Token`
- **(graphql-minify)** get rid of multiple memory allocations in `print_block_string`
- **(graphql-minify)** use `memchr` for string validation
