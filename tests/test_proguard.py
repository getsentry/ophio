import os
import uuid

from sentry_ophio.proguard import ProguardMapper

def test_mapper(fixture_path):
    mapper = ProguardMapper.open(os.path.join(fixture_path, "proguard.txt"))
    assert mapper.has_line_info
    # FIXME:
    assert mapper.uuid == uuid.UUID("da86dcce-c05f-5cc1-a1a0-8fbf5d74beb2")

    assert (
        mapper.remap_class("android.support.constraint.ConstraintLayout$a")
        == "android.support.constraint.ConstraintLayout$LayoutParams"
    )

    assert mapper.remap_method("android.support.constraint.a.b", "f") == (
        "android.support.constraint.solver.ArrayRow",
        "pickRowVariable",
    )

    remapped = mapper.remap_frame("android.support.constraint.a.b", "a", 116)
    assert len(remapped) == 1
    assert remapped[0].class_name == "android.support.constraint.solver.ArrayRow"
    assert remapped[0].method == "createRowDefinition"
    assert remapped[0].line == 116

    remapped = mapper.remap_frame("io.sentry.sample.MainActivity", "a", 1)
    assert len(remapped) == 3
    assert remapped[0].method == "bar"
    assert remapped[0].line == 54
    assert remapped[1].method == "foo"
    assert remapped[1].line == 44
    assert remapped[2].method == "onClickHandler"
    assert remapped[2].line == 40