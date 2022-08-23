# blau

This repository contains:
 - a Rust implementation of the core Blau game logic
 - a Python binding to the Rust code
 - a self-play script for testing Blau agents

## Usage

Test the Rust code:

```
cargo test
```

Build and install the Python package:

```
pip install --user -e .
```

Run the self-play script (requires extra deps):

```
pip install --user pandas matplotlib scipy
python3 scripts/self_play.py --plot
```

### Build wheels

```
sudo docker run --rm -v `pwd`:/io quay.io/pypa/manylinux2010_x86_64 /io/build-wheels.sh
```
