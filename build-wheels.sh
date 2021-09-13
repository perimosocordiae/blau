#!/bin/bash
set -ex

curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y
export PATH="$HOME/.cargo/bin:$PATH"

cd /io

for PYBIN in /opt/python/cp3{6,7,8,9,10}*/bin; do
    "${PYBIN}/pip" install -U setuptools wheel setuptools-rust
    "${PYBIN}/python" setup.py bdist_wheel
done

for whl in dist/*.whl; do
    auditwheel repair "$whl"
done

