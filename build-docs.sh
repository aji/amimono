#!/bin/sh

cargo doc --no-deps -p amimono && rsync -aP target/doc/ docs
