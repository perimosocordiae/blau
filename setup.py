from setuptools import setup, Distribution
from setuptools_rust import RustExtension, Binding

setup(
    name='blau',
    version='0.1.0',
    author='CJ Carey',
    packages=['blau'],
    zip_safe=False,
    rust_extensions=[
        RustExtension('blau', 'Cargo.toml', debug=False, binding=Binding.NoBinding),
    ],
)

