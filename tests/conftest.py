import os
import pytest


@pytest.fixture(scope="module")
def fixture_path():
    here = os.path.abspath(os.path.dirname(__file__))
    return os.path.join(here, "fixtures")
