#!/bin/bash
set -ex

curl -sSf https://sh.rustup.rs | sh -s -- -y
export PATH="$HOME/.cargo/bin:$PATH"

cd /io

for PYBIN in /opt/python/cp3{6,7,8,9,10}*/bin; do
    rm -rf build/
    "${PYBIN}/pip" install -U setuptools wheel setuptools-rust
    "${PYBIN}/python" setup.py bdist_wheel
done

for whl in dist/*.whl; do
    auditwheel repair "$whl"
done

cd /
for PYBIN in /opt/python/cp3{6,7,8,9,10}*/bin; do
    "${PYBIN}/pip" install blau -f /io/wheelhouse/
    "${PYBIN}/python" -c 'import blau; blau.BlauMove(0,1,2)'
done
