#!/bin/bash

# build the programs
cd hub
cargo build --release
cd ../spoke
cargo build --release


# run the dockercontainer
docker-compose down
docker-compose up --build
