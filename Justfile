import ".just/cargo.just"
import ".just/commit.just"
import ".just/ship.just"

default: pc-fix clippy test

pc-fix:
  prek run --all-files
