import os
import urllib.request
from time import sleep

import pytest

@pytest.fixture
def test_file_path():
    file_path = "tests/markdown_files/test.md"
    yield file_path
    if os.path.exists(file_path):
        os.remove(file_path)

def test_file_tracking(test_file_path: str):
    with urllib.request.urlopen("http://127.0.0.1:4000/health") as response:
        assert response.getcode() == 200

    # File doesn't exist
    with pytest.raises(urllib.request.HTTPError):
        with urllib.request.urlopen("http://127.0.0.1:4000/frontmatter/file/test.md") as response:
            assert response.getcode() == 404

    with open(test_file_path, "x") as f:
        f.write("Just call me mark!\n")
        f.flush()
    sleep(1)

    # Custard recognizes created file
    with urllib.request.urlopen("http://127.0.0.1:4000/frontmatter/file/test.md") as response:
        assert response.getcode() == 200
    with open(test_file_path, "a") as f:
        f.write("I'm a markdown file!\n")
        f.flush()
    sleep(1)

    # Custard has updated on file edit
    with urllib.request.urlopen("http://127.0.0.1:4000/frontmatter/file/test.md") as response:
        assert response.read().decode() == """Just call me mark!
I'm a markdown file!
"""
    os.remove(test_file_path)
    sleep(1)

    # Custard recognizes file deleted
    with pytest.raises(urllib.request.HTTPError):
        with urllib.request.urlopen("http://127.0.0.1:4000/frontmatter/file/test.md") as response:
            assert response.getcode() == 404
    
