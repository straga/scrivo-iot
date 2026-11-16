"""The contract as a document and as the board keeps it are one number (docs/CONTRACT_Device_Api.md):
the document, the burner's dictionary and the page read the same contract.

    uv run --no-project --with pytest pytest -q tools/test_device_contract.py
"""
import os
import re

from page_stand import dictionary_of

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)


def documented():
    text = open(os.path.join(ROOT, "docs", "CONTRACT_Device_Api.md")).read()
    found = re.search(r"Contract version: `(\d+)`", text)
    assert found, "the document says no number"
    return int(found.group(1))


def test_the_burners_dictionary_keeps_the_contract_the_document_is():
    assert dictionary_of(ROOT)["contract"] == documented() == 10


def test_a_dictionary_of_another_contract_is_one_the_document_does_not_describe():
    dictionary = dictionary_of(ROOT)
    assert dict(dictionary, contract=1)["contract"] != documented()


def test_every_example_of_the_contract_in_the_document_says_the_number_the_document_is():
    # the example of /state said 4 under a document of 5
    text = open(os.path.join(ROOT, "docs", "CONTRACT_Device_Api.md")).read()
    said = [int(n) for n in re.findall(r'"contract": (\d+)', text)]
    assert said, "the document has no example that says a contract"
    assert set(said) == {documented()}, said
