from setuptools import setup
from setuptools_rust import RustExtension, Binding

setup(
    name="blau",
    version="0.1.2",
    author="CJ Carey",
    packages=["blau"],
    zip_safe=False,
    rust_extensions=[
        RustExtension(
            "blau.blau", debug=False, binding=Binding.PyO3, features=["pyo3"]
        ),
    ],
    package_data=dict(blau=["blau.pyi"]),
)
