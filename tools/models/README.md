# Models

`python3 tools/models/build.py` writes `data/models.json` from `parts/*.py`; `common.py` holds the helpers and explains a part. The core draws them (`crates/core/src/look.rs`), with an ink outline on every solid. To look at them before the game draws them, run the preview in the journal: `mtg-hand-of-fate-journal/mockups/models.html` reads this repository's `data/models.json` with the same meshes and rotation order (`python3 mockups/shoot_models.py <name> ...` from the journal folder saves pictures).

Conventions: centimetres; y up; a model stands on y = 0 and faces +z. A person is about 180 cm. Creatures keep their own sizes (a 6/7 Axebane Stag is big); the battle scales them for the arena. Keep a model under 120 parts. Low-poly and rounded, after Protection of the Spheres and Mage Tower Battle: bodies of spheres and blobs, faces with eyes, a few cones and cylinders. Colour by the card's art.
