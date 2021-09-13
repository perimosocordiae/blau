from setuptools import setup
from setuptools_rust import RustExtension, Binding

setup(
    name='blau',
    version='0.1.0',
    author='CJ Carey',
    packages=['blau'],
    zip_safe=False,
    rust_extensions=[
        RustExtension('blau.blau', debug=False, binding=Binding.RustCPython),
    ],
    package_data=dict(blau=['blau.pyi']),
)
