#!/bin/sh

mvn -f ./java/pom.xml --batch-mode --update-snapshots install
mvn -f ./java/pom.xml --batch-mode --update-snapshots install
mvn -f ./test-resources/java/pom.xml --batch-mode --update-snapshots install
cargo test --manifest-path ./rust/Cargo.toml -- $@
